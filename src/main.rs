use crate::cli::Opts;
use anyhow::{anyhow, Context, Result as AnyhowResult};
use clap::Parser;
use git2::build::RepoBuilder;
use git2::{
    Direction, Error, IndexAddOption, ObjectType, PushOptions, Remote, Repository, Signature,
    Status, StatusOptions,
};
use log::{error, info, warn};
use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::str;
use std::time::Duration;

pub mod cli;
pub mod operation;

fn main() {
    env_logger::init();
    let opts: Opts = Opts::parse();

    let repository = match Repository::open(&opts.path) {
        Ok(repo) => repo,
        Err(e) => {
            eprint!("folder does not exist or is not a valid git folder: {}", e);
            return;
        }
    };

    loop {
        if let Err(e) = inner_loop(&opts, &repository) {
            warn!("interval run throw an error: {}", e);
        }
        std::thread::sleep(Duration::from_secs(5));
    }
}

fn inner_loop(opts: &Opts, repository: &Repository) -> AnyhowResult<()> {
    let mut status_options = StatusOptions::new();
    status_options.include_ignored(false);
    status_options
        .include_untracked(true)
        .recurse_untracked_dirs(true);

    let status = repository.statuses(Some(&mut status_options))?;

    let object = repository.head()?.resolve()?.peel(ObjectType::Commit)?;
    let last_commit = object
        .into_commit()
        .map_err(|_| anyhow!("it is not a commit"))?;

    info!("last commit is {}", last_commit.id());

    let mut has_new_commit = false;
    dbg!(status.is_empty());
    if !status.is_empty() {
        let mut index = repository.index()?;
        let cb = &mut |path: &Path, _matched_spec: &[u8]| -> i32 {
            info!("add {}", path.display());
            0
        };
        for status_entity in status.iter() {
            if status_entity.status() != Status::CURRENT {
                let buf =
                    PathBuf::from(String::from_utf8_lossy(status_entity.path_bytes()).to_string());
                index.add_all([buf].iter(), IndexAddOption::DEFAULT, Some(cb))?
            }
        }
        index.write()?;
        let x = index.write_tree()?;
        dbg!(&x);
        let tree = repository.find_tree(x)?;
        let default_signature = repository.signature()?;
        let signature = Signature::now(
            opts.author_name
                .as_deref()
                .or(default_signature.name())
                .unwrap_or("Fis Committer"),
            opts.author_email
                .as_deref()
                .or(default_signature.email())
                .unwrap_or("fis@bot.com"),
        )?;

        let object = repository.head()?.resolve()?.peel(ObjectType::Commit)?;
        let last_commit = object
            .into_commit()
            .map_err(|_| anyhow!("it is not a commit"))?;

        info!(
            "tree id : {} head commit id: {}",
            tree.id(),
            last_commit.id()
        );
        repository.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Committed by fis",
            &tree,
            &[&last_commit],
        )?;
        has_new_commit = true;
    }

    // pull
    let mut remote = repository.find_remote("origin")?;
    let commit = operation::do_fetch(&repository, &["main"], &mut remote)?;
    operation::do_merge(&repository, "main", commit)?;

    // push
    if has_new_commit {
        let mut remote = repository.find_remote("origin")?;
        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(operation::git_pw_credentials_callback);

        remote.connect_auth(Direction::Push, Some(callbacks), None)?;

        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(operation::git_pw_credentials_callback);

        let mut options = PushOptions::new();
        options.remote_callbacks(callbacks);
        remote.push(&["refs/heads/main:refs/heads/main"], Some(&mut options))?;
    }

    Ok(())
}
