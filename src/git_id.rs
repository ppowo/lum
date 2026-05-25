use anyhow::{Context, Result};

use crate::cli::GitIdCommand;
use crate::shell;

#[path = "git_id/artifacts.rs"]
mod artifacts;
#[path = "git_id/config.rs"]
mod config;

use crate::paths::expand_path;
use artifacts::{
    cleanup_old_backups, cleanup_orphan_identity_configs, cleanup_orphan_key_pairs,
    is_lum_managed_file, replace_marked_section,
};
pub use config::config_path;
use config::{
    GitIdentitiesConfig, Identity, allowed_signers_path, data_dir, detect_identity, git_path,
    identity_git_config_path, identity_private_key_path, identity_public_key_path, load_config,
};

pub fn run(command: GitIdCommand) -> Result<()> {
    match command {
        GitIdCommand::ConfigPath => {
            println!("{}", shell::quote_path(&config_path()?));
            Ok(())
        }
        GitIdCommand::Init => init(),
        GitIdCommand::Where => where_am_i(),
        GitIdCommand::Sync => sync(),
        GitIdCommand::Status => status(),
        GitIdCommand::Info { identity } => info(&identity),
        GitIdCommand::Pubkey { identity } => pubkey(&identity),
        GitIdCommand::Paths => paths(),
    }
}

fn init() -> Result<()> {
    let path = config_path()?;
    if path.exists() {
        println!("{} already exists, not overwriting", path.display());
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let sample = GitIdentitiesConfig {
        identities: vec![Identity {
            name: "github-work".into(),
            author_name: "Jane Doe".into(),
            email: "jane@company.com".into(),
            domain: "github.com".into(),
            folders: vec!["~/Work/Github".into()],
        }],
    };
    std::fs::write(&path, serde_json::to_string_pretty(&sample)?)?;
    println!("Created {} with a sample git identity", path.display());
    Ok(())
}

fn where_am_i() -> Result<()> {
    let identities = load_config()?;
    let cwd = std::env::current_dir()?;
    let identity = detect_identity(&identities, &cwd)
        .ok_or_else(|| anyhow::anyhow!("no git identity detected for {}", cwd.display()))?;
    println!("Identity: {}", identity.name);
    println!("Author:   {}", identity.author_name);
    println!("Email:    {}", identity.email);
    println!("Domain:   {}", identity.domain);
    Ok(())
}

fn sync() -> Result<()> {
    let identities = load_config()?;
    ensure_ssh_keygen_on_path()?;
    cleanup_old_backups()?;

    let mut failures = Vec::new();
    for identity in &identities {
        if let Err(error) = sync_identity(identity) {
            failures.push(format!("{}: {error:#}", identity.name));
            eprintln!("{}: {error:#}", identity.name);
        }
    }

    write_global_git_config(&identities)?;
    write_ssh_config(&identities)?;
    write_allowed_signers(&identities)?;
    cleanup_orphan_identity_configs(&identities, identity_git_config_path)?;
    cleanup_orphan_key_pairs(&identities)?;

    if !failures.is_empty() {
        anyhow::bail!("{} git identity sync failure(s)", failures.len());
    }
    Ok(())
}

fn ensure_ssh_keygen_on_path() -> Result<()> {
    match std::process::Command::new("ssh-keygen").arg("-h").output() {
        Ok(_) => Ok(()),
        Err(_) => anyhow::bail!("ssh-keygen executable not found on PATH"),
    }
}

fn sync_identity(identity: &Identity) -> Result<()> {
    for folder in &identity.folders {
        std::fs::create_dir_all(expand_path(folder))?;
    }
    ensure_key_pair(identity)?;
    write_identity_git_config(identity)?;
    println!("{}", identity.name);
    Ok(())
}

fn ensure_key_pair(identity: &Identity) -> Result<()> {
    let private_key = identity_private_key_path(identity)?;
    let public_key = identity_public_key_path(identity)?;

    if private_key.exists() || public_key.exists() {
        ensure_lum_managed_key(identity, &public_key)?;
        return Ok(());
    }

    if let Some(parent) = private_key.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let comment = format!("{} [lum:git-id identity={}]", identity.email, identity.name);
    let output = std::process::Command::new("ssh-keygen")
        .args(["-t", "ed25519", "-C", &comment, "-f"])
        .arg(&private_key)
        .args(["-N", ""])
        .output()
        .with_context(|| "running ssh-keygen")?;
    if !output.status.success() {
        anyhow::bail!(
            "ssh-keygen failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    ensure_lum_managed_key(identity, &public_key)?;
    println!("  public key: {}", public_key.display());
    Ok(())
}

fn ensure_lum_managed_key(identity: &Identity, public_key: &std::path::Path) -> Result<()> {
    let marker = format!("[lum:git-id identity={}]", identity.name);
    let content = std::fs::read_to_string(public_key)
        .with_context(|| format!("reading public key {}", public_key.display()))?;
    if !content.contains(&marker) {
        anyhow::bail!(
            "refusing to touch unmarked key at {} (missing {})",
            public_key.display(),
            marker
        );
    }
    Ok(())
}

fn write_identity_git_config(identity: &Identity) -> Result<()> {
    let path = identity_git_config_path(identity)?;
    if path.exists()
        && !is_lum_managed_file(
            &path,
            &format!("# lum:git-id:managed identity={}", identity.name),
        )?
    {
        anyhow::bail!(
            "refusing to overwrite unmarked git config at {}",
            path.display()
        );
    }
    let private_key = git_path(&identity_private_key_path(identity)?);
    let public_key = git_path(&identity_public_key_path(identity)?);
    let allowed_signers = git_path(&allowed_signers_path()?);
    let content = format!(
        "# lum:git-id:managed identity={}\n\n[user]\n  name = {}\n  email = {}\n  signingkey = {}\n\n[core]\n  sshCommand = \"ssh -i {} -o IdentitiesOnly=yes\"\n\n[commit]\n  gpgsign = true\n\n[gpg]\n  format = ssh\n\n[gpg \"ssh\"]\n  allowedSignersFile = {}\n\n[url \"ssh://git@{}/\"]\n  insteadOf = https://{}/\n",
        identity.name,
        identity.author_name,
        identity.email,
        public_key,
        private_key,
        allowed_signers,
        identity.domain,
        identity.domain
    );
    std::fs::write(path, content)?;
    Ok(())
}

fn write_global_git_config(identities: &[Identity]) -> Result<()> {
    let path = crate::paths::home_path(".gitconfig")?;
    let mut entries: Vec<_> = identities
        .iter()
        .flat_map(|identity| {
            identity
                .folders
                .iter()
                .map(move |folder| (identity, folder))
        })
        .collect();
    entries.sort_by_key(|(_, folder)| {
        crate::paths::normalize_path(&expand_path(folder))
            .components()
            .count()
    });

    let mut section = String::from("# lum:git-id:begin\n");
    for (identity, folder) in entries {
        let mut folder_path = git_path(&expand_path(folder));
        if !folder_path.ends_with('/') {
            folder_path.push('/');
        }
        section.push_str(&format!(
            "[includeIf \"gitdir:{}\"]\n  path = {}\n\n",
            folder_path,
            git_path(&identity_git_config_path(identity)?)
        ));
    }
    section.push_str("# lum:git-id:end\n");
    replace_marked_section(&path, "# lum:git-id:begin", "# lum:git-id:end", &section)
}

fn write_ssh_config(identities: &[Identity]) -> Result<()> {
    let path = crate::paths::home_path(".ssh/config")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut seen = std::collections::HashSet::new();
    let mut section = String::from("# lum:git-id:begin\n");
    for identity in identities {
        if seen.insert(identity.domain.clone()) {
            section.push_str(&format!(
                "Host {}\n  HostName {}\n  User git\n  IdentityFile {}\n  IdentitiesOnly yes\n\n",
                identity.domain,
                identity.domain,
                git_path(&identity_private_key_path(identity)?)
            ));
        }
    }
    section.push_str("# lum:git-id:end\n");
    replace_marked_section(&path, "# lum:git-id:begin", "# lum:git-id:end", &section)
}

fn write_allowed_signers(identities: &[Identity]) -> Result<()> {
    let path = allowed_signers_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut section = String::from("# lum:git-id:begin\n");
    for identity in identities {
        let public_key = identity_public_key_path(identity)?;
        if let Ok(content) = std::fs::read_to_string(public_key) {
            let fields: Vec<_> = content.split_whitespace().collect();
            if fields.len() >= 2 {
                section.push_str(&format!("{} {} {}\n", identity.email, fields[0], fields[1]));
            }
        }
    }
    section.push_str("# lum:git-id:end\n");
    replace_marked_section(&path, "# lum:git-id:begin", "# lum:git-id:end", &section)
}

fn status() -> Result<()> {
    for identity in load_config()? {
        let key = identity_public_key_path(&identity)?;
        let status = if key.exists() { "active" } else { "needs sync" };
        println!("{}  {}", identity.name, status);
    }
    Ok(())
}

fn info(name: &str) -> Result<()> {
    let identities = load_config()?;
    let identity = identities
        .iter()
        .find(|identity| identity.name == name)
        .ok_or_else(|| anyhow::anyhow!("identity not found: {name}"))?;
    println!("Identity: {}", identity.name);
    println!("Author:   {}", identity.author_name);
    println!("Email:    {}", identity.email);
    println!("Domain:   {}", identity.domain);
    println!(
        "Git config: {}",
        identity_git_config_path(identity)?.display()
    );
    println!(
        "Public key: {}",
        identity_public_key_path(identity)?.display()
    );
    Ok(())
}

fn pubkey(name: &str) -> Result<()> {
    let identities = load_config()?;
    let identity = identities
        .iter()
        .find(|identity| identity.name == name)
        .ok_or_else(|| anyhow::anyhow!("identity not found: {name}"))?;
    let path = identity_public_key_path(identity)?;
    let content = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "reading public key {} (run lum git-id sync)",
            path.display()
        )
    })?;
    print!("{}", content);
    Ok(())
}

fn paths() -> Result<()> {
    let identities = if config_path()?.exists() {
        load_config()?
    } else {
        vec![]
    };
    println!("Config:\n  {}", config_path()?.display());
    println!("Global files touched:");
    println!(
        "  {}",
        crate::paths::home_dir()
            .unwrap_or_default()
            .join(".gitconfig")
            .display()
    );
    println!(
        "  {}",
        crate::paths::home_dir()
            .unwrap_or_default()
            .join(".ssh/config")
            .display()
    );
    println!(
        "  {}",
        crate::paths::home_dir()
            .unwrap_or_default()
            .join(".ssh/allowed_signers")
            .display()
    );
    println!("Backups:\n  {}", data_dir()?.join("backups").display());
    if !identities.is_empty() {
        println!("Identities:");
        for identity in identities {
            println!("  {}", identity.name);
            println!(
                "    git config: {}",
                identity_git_config_path(&identity)?.display()
            );
            println!(
                "    private key: {}",
                identity_private_key_path(&identity)?.display()
            );
            println!(
                "    public key: {}",
                identity_public_key_path(&identity)?.display()
            );
            for folder in identity.folders {
                println!("    folder: {}", folder);
            }
        }
    }
    Ok(())
}
