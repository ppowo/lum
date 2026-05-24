use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::cli::GitIdCommand;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub name: String,
    pub author_name: String,
    pub email: String,
    pub domain: String,
    pub folders: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitIdentitiesConfig {
    pub identities: Vec<Identity>,
}

pub fn run(command: GitIdCommand) -> Result<()> {
    match command {
        GitIdCommand::ConfigPath => {
            println!("{}", config_path()?.display());
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

pub fn config_path() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", "lum")
        .ok_or_else(|| anyhow::anyhow!("cannot determine config directory"))?;
    Ok(dirs.config_dir().join("git-identities.json"))
}

fn data_dir() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", "lum")
        .ok_or_else(|| anyhow::anyhow!("cannot determine data directory"))?;
    Ok(dirs.data_dir().join("git-id"))
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

fn load_config() -> Result<Vec<Identity>> {
    let path = config_path()?;
    let content =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let config: GitIdentitiesConfig =
        serde_json::from_str(&content).with_context(|| format!("parsing {}", path.display()))?;
    validate(&config.identities)?;
    Ok(config.identities)
}

fn validate(identities: &[Identity]) -> Result<()> {
    let mut names = HashSet::new();
    let mut folders = HashSet::new();
    let mut email_domains = HashSet::new();
    let mut author_domains = HashSet::new();

    for identity in identities {
        if identity.name.trim().is_empty() {
            anyhow::bail!("identity name must not be empty");
        }
        if !names.insert(identity.name.clone()) {
            anyhow::bail!("duplicate identity name: {}", identity.name);
        }
        if identity.author_name.trim().is_empty() {
            anyhow::bail!("identity {}: author_name must not be empty", identity.name);
        }
        if identity.email.trim().is_empty() {
            anyhow::bail!("identity {}: email must not be empty", identity.name);
        }
        if identity.domain.trim().is_empty() {
            anyhow::bail!("identity {}: domain must not be empty", identity.name);
        }
        if identity.folders.is_empty() {
            anyhow::bail!(
                "identity {}: at least one folder is required",
                identity.name
            );
        }
        let email_domain = (identity.email.clone(), identity.domain.clone());
        if !email_domains.insert(email_domain) {
            anyhow::bail!(
                "duplicate email+domain: {} on {}",
                identity.email,
                identity.domain
            );
        }
        let author_domain = (identity.author_name.clone(), identity.domain.clone());
        if !author_domains.insert(author_domain) {
            anyhow::bail!(
                "duplicate author_name+domain: {} on {}",
                identity.author_name,
                identity.domain
            );
        }
        for folder in &identity.folders {
            if folder.trim().is_empty() {
                anyhow::bail!("identity {}: folder must not be empty", identity.name);
            }
            let expanded = expand_path(folder);
            let normalized = normalize_path(&expanded);
            if !folders.insert(normalized) {
                anyhow::bail!("duplicate managed folder: {}", folder);
            }
        }
    }
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

fn detect_identity<'a>(identities: &'a [Identity], dir: &Path) -> Option<&'a Identity> {
    let dir = normalize_path(dir);
    identities
        .iter()
        .filter_map(|identity| {
            identity
                .folders
                .iter()
                .map(|folder| normalize_path(&expand_path(folder)))
                .filter(|folder| is_path_prefix(folder, &dir))
                .map(|folder| (folder.components().count(), identity))
                .max_by_key(|(len, _)| *len)
        })
        .max_by_key(|(len, _)| *len)
        .map(|(_, identity)| identity)
}

fn is_path_prefix(prefix: &Path, path: &Path) -> bool {
    path == prefix || path.starts_with(prefix)
}

fn expand_path(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

fn normalize_path(path: &Path) -> PathBuf {
    path.components().collect()
}

fn identity_private_key_path(identity: &Identity) -> Result<PathBuf> {
    Ok(dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?
        .join(".ssh")
        .join(format!("lum-git-id-{}", identity.name)))
}

fn identity_public_key_path(identity: &Identity) -> Result<PathBuf> {
    Ok(identity_private_key_path(identity)?.with_extension("pub"))
}

fn identity_git_config_path(identity: &Identity) -> Result<PathBuf> {
    Ok(dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?
        .join(format!(".gitconfig-lum-git-id-{}", identity.name)))
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
    cleanup_orphan_identity_configs(&identities)?;
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

fn ensure_lum_managed_key(identity: &Identity, public_key: &Path) -> Result<()> {
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
    let path = home_path(".gitconfig")?;
    let existing = read_optional(&path)?;
    let stripped = strip_marked_section(&existing, "# lum:git-id:begin", "# lum:git-id:end");
    let mut entries: Vec<_> = identities
        .iter()
        .flat_map(|identity| {
            identity
                .folders
                .iter()
                .map(move |folder| (identity, folder))
        })
        .collect();
    entries.sort_by_key(|(_, folder)| normalize_path(&expand_path(folder)).components().count());

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
    std::fs::write(path, append_managed_section(&stripped, &section))?;
    Ok(())
}

fn write_ssh_config(identities: &[Identity]) -> Result<()> {
    let path = home_path(".ssh/config")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let existing = read_optional(&path)?;
    let stripped = strip_marked_section(&existing, "# lum:git-id:begin", "# lum:git-id:end");
    let mut seen = HashSet::new();
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
    std::fs::write(path, append_managed_section(&stripped, &section))?;
    Ok(())
}

fn write_allowed_signers(identities: &[Identity]) -> Result<()> {
    let path = allowed_signers_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let existing = read_optional(&path)?;
    let stripped = strip_marked_section(&existing, "# lum:git-id:begin", "# lum:git-id:end");
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
    std::fs::write(path, append_managed_section(&stripped, &section))?;
    Ok(())
}

fn allowed_signers_path() -> Result<PathBuf> {
    home_path(".ssh/allowed_signers")
}

fn home_path(relative: &str) -> Result<PathBuf> {
    Ok(dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?
        .join(relative))
}

fn read_optional(path: &Path) -> Result<String> {
    match std::fs::read_to_string(path) {
        Ok(content) => Ok(content),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(error.into()),
    }
}

fn strip_marked_section(content: &str, begin: &str, end: &str) -> String {
    let mut output = String::new();
    let mut in_section = false;
    for line in content.lines() {
        if line.trim() == begin {
            in_section = true;
            continue;
        }
        if line.trim() == end {
            in_section = false;
            continue;
        }
        if !in_section {
            output.push_str(line);
            output.push('\n');
        }
    }
    output
}

fn append_managed_section(existing_without_section: &str, section: &str) -> String {
    let existing = existing_without_section.trim_end();
    if existing.is_empty() {
        section.to_string()
    } else {
        format!("{existing}\n\n{section}")
    }
}

fn is_lum_managed_file(path: &Path, marker: &str) -> Result<bool> {
    Ok(read_optional(path)?.starts_with(marker))
}

fn git_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn cleanup_old_backups() -> Result<()> {
    let backup_dir = data_dir()?.join("backups");
    if !backup_dir.exists() {
        return Ok(());
    }
    let cutoff = std::time::SystemTime::now() - std::time::Duration::from_secs(30 * 24 * 60 * 60);
    for entry in std::fs::read_dir(backup_dir)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.modified().unwrap_or(std::time::SystemTime::now()) < cutoff {
            let _ = std::fs::remove_file(entry.path());
        }
    }
    Ok(())
}

fn cleanup_orphan_identity_configs(identities: &[Identity]) -> Result<()> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
    let active: HashSet<_> = identities
        .iter()
        .map(|identity| identity.name.clone())
        .collect();
    for entry in std::fs::read_dir(&home)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(identity_name) = name.strip_prefix(".gitconfig-lum-git-id-") {
            if !active.contains(identity_name)
                && is_lum_managed_file(
                    &entry.path(),
                    &format!("# lum:git-id:managed identity={identity_name}"),
                )?
            {
                backup_and_remove(&[entry.path()])?;
            }
        }
    }
    Ok(())
}

fn cleanup_orphan_key_pairs(identities: &[Identity]) -> Result<()> {
    let ssh_dir = home_path(".ssh")?;
    if !ssh_dir.exists() {
        return Ok(());
    }
    let active: HashSet<_> = identities
        .iter()
        .map(|identity| identity.name.clone())
        .collect();
    for entry in std::fs::read_dir(&ssh_dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(identity_name) = file_name
            .strip_prefix("lum-git-id-")
            .and_then(|name| name.strip_suffix(".pub"))
        else {
            continue;
        };
        if active.contains(identity_name) {
            continue;
        }
        let marker = format!("[lum:git-id identity={identity_name}]");
        let content = read_optional(&path)?;
        if !content.contains(&marker) {
            continue;
        }
        let private_key = ssh_dir.join(format!("lum-git-id-{identity_name}"));
        let mut paths = vec![path];
        if private_key.exists() {
            paths.push(private_key);
        }
        backup_and_remove(&paths)?;
    }
    Ok(())
}

fn backup_and_remove(paths: &[PathBuf]) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    let backup_dir = data_dir()?.join("backups");
    std::fs::create_dir_all(&backup_dir)?;
    let timestamp = chrono_like_timestamp();
    let backup_path = backup_dir.join(format!("git-id-orphans-{timestamp}.tar.gz"));
    let file = std::fs::File::create(&backup_path)?;
    let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut tar = tar::Builder::new(encoder);
    for path in paths {
        if path.exists() {
            tar.append_path_with_name(path, path.file_name().unwrap_or_default())?;
        }
    }
    tar.finish()?;
    for path in paths {
        let _ = std::fs::remove_file(path);
    }
    Ok(())
}

fn chrono_like_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    seconds.to_string()
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
        dirs::home_dir()
            .unwrap_or_default()
            .join(".gitconfig")
            .display()
    );
    println!(
        "  {}",
        dirs::home_dir()
            .unwrap_or_default()
            .join(".ssh/config")
            .display()
    );
    println!(
        "  {}",
        dirs::home_dir()
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
