use std::{
    fs::{self, create_dir_all, read_dir},
    path::{Path, PathBuf},
};

use chrono::DateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::{
    config::{self, Config},
    util,
};

#[derive(Debug, Serialize)]
pub struct Backup {
    file_info: Vec<FileInfo>,
    backup_info: BackupInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupInfo {
    pub timestamp: DateTime<chrono::Utc>,
    pub path_to_root: PathBuf,
    pub backup_prefix: String,
}

impl BackupInfo {
    fn get_path(&self) -> Option<(BackupInfo, PathBuf)> {
        let config = Config::read_config();
        let backup_info_path = config.get_default_backup_info_path();
        let backup_info = Self::get_backup_info_by_path(backup_info_path);
        backup_info
            .into_iter()
            .find(|(info, _)| info.backup_prefix == self.backup_prefix)
    }
    fn get_backup_info_by_path(backup_info_path: String) -> Vec<(BackupInfo, PathBuf)> {
        match read_dir(&backup_info_path) {
            Ok(entries) => entries
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| {
                    let content = fs::read_to_string(entry.path()).ok()?;
                    let info: BackupInfo = serde_json::from_str(&content).ok()?;
                    Some((info, entry.path()))
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    }
}

impl Backup {
    pub fn new(root_dir: PathBuf) -> anyhow::Result<Self> {
        let prefix = generate_prefix(&root_dir);
        Ok(Self {
            file_info: Self::build_info(&root_dir, &prefix)?,
            backup_info: BackupInfo {
                backup_prefix: prefix,
                path_to_root: root_dir,
                timestamp: chrono::Utc::now(),
            },
        })
    }

    pub fn write_backup(&mut self) -> anyhow::Result<()> {
        // If no changes detected, skip backup creation
        if self.file_info.is_empty() {
            println!("No changes detected. Skipping backup creation.");
            return Ok(());
        }

        let backup = serde_json::to_string_pretty(&self.file_info)?;
        let backup_info = serde_json::to_string_pretty(&self.backup_info)?;
        let config = Config::read_config();
        let backup_info_path = config.get_default_backup_info_path();

        match self.backup_info.get_path() {
            Some(path) => {
                fs::write(path.1, backup_info)?;
            }
            None => {
                let backup_info_file_path = Path::new(&backup_info_path)
                    .join(&self.backup_info.backup_prefix)
                    .with_extension("json");
                create_dir_all(backup_info_file_path.parent().unwrap())?;
                fs::write(backup_info_file_path, backup_info)?;
            }
        }

        let binding = config::Config::read_config();
        let backup_path = binding.get_default_backup_path();
        let next_backup_path =
            Self::next_backup_file(&PathBuf::from(backup_path), &self.backup_info.backup_prefix);
        create_dir_all(Path::new(&next_backup_path).parent().unwrap())?;
        fs::write(&next_backup_path, backup)?;

        println!(
            "Backup created with {} changes at: {}",
            self.file_info.len(),
            next_backup_path.display()
        );
        Ok(())
    }

    fn next_backup_file(backup_dir: &PathBuf, prefix: &str) -> PathBuf {
        if let Err(_) = create_dir_all(backup_dir) {
            return backup_dir.join(prefix).join("backup_0.json");
        }

        let backup_files = Self::get_backup_files_by_prefix(backup_dir, prefix);

        if backup_files.is_empty() {
            return backup_dir.join(prefix).join("backup_0.json");
        }

        let max_number = backup_files
            .iter()
            .filter_map(|path| {
                let file_name = path.file_name()?.to_string_lossy();
                if file_name.starts_with("backup_") && file_name.ends_with(".json") {
                    let number_part = file_name
                        .trim_start_matches("backup_")
                        .trim_end_matches(".json");
                    number_part.parse::<u32>().ok()
                } else {
                    None
                }
            })
            .max()
            .unwrap_or(0);

        backup_dir
            .join(prefix)
            .join(format!("backup_{}.json", max_number + 1))
    }

    fn build_info(path: &PathBuf, prefix: &str) -> anyhow::Result<Vec<FileInfo>> {
        let backup = Self::get_backup(prefix);
        match backup {
            Some(backup_info) => {
                let file_infos = Self::process_exits_backup(&backup_info.backup_prefix, path);
                file_infos
            }
            None => {
                let mut file_infos = Vec::new();

                for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                    let path = entry.path();

                    // Обробляємо тільки файли
                    if path.is_file() {
                        let size = fs::metadata(&path)?.len();
                        let hash = util::hash::calculate_file_hash(&path)?;

                        // Для першого backup'а зберігаємо контент всіх файлів
                        let backup_path = config::Config::read_config().get_default_backup_path();
                        let content_path = match FileInfo::store_content(
                            &path.to_string_lossy(),
                            &hash,
                            &PathBuf::from(&backup_path),
                            ContentType::FullCopy,
                        ) {
                            Ok(path) => Some(path),
                            Err(e) => {
                                println!("Failed to store content for {}: {}", path.display(), e);
                                None
                            }
                        };
                        
                        file_infos.push(FileInfo::new(
                            path.to_string_lossy().to_string(),
                            size,
                            hash,
                            chrono::Utc::now(),
                            false,
                            ContentType::FullCopy,
                            content_path,
                        ));
                    }
                }
                Ok(file_infos)
            }
        }
    }
    fn get_backup(prefix: &str) -> Option<BackupInfo> {
        let config = Config::read_config();
        let backup_info_path = config.get_default_backup_info_path();

        let backup_info = BackupInfo::get_backup_info_by_path(backup_info_path);
        backup_info
            .into_iter()
            .find(|(info, _)| info.backup_prefix == prefix)
            .map(|(info, _)| info)
    }

    fn get_backup_files_by_prefix(backup_dir: &PathBuf, prefix: &str) -> Vec<PathBuf> {
        let prefix_dir = backup_dir.join(prefix);

        match read_dir(&prefix_dir) {
            Ok(entries) => entries
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    if file_name.starts_with("backup_") && file_name.ends_with(".json") {
                        Some(entry.path())
                    } else {
                        None
                    }
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    fn process_exits_backup(prefix: &str, path: &PathBuf) -> anyhow::Result<Vec<FileInfo>> {
        let backup_path = config::Config::read_config().get_default_backup_path();
        let files = Self::get_backup_files_by_prefix(&PathBuf::from(&backup_path), prefix);
        println!("files -> {:#?}", files);
        let file_infos = FileInfo::get_vec_file_info_by_paths(files);

        let mut file_info_new = Vec::new();
        let mut processed_paths = std::collections::HashSet::new();

        // Обробляємо поточні файли
        for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            let current_path = entry.path();
            if current_path.is_file() {
                let current_path_str = current_path.to_string_lossy().to_string();
                processed_paths.insert(current_path_str.clone());

                // Знаходимо найновіший запис про цей файл
                let latest_file_record = file_infos
                    .iter()
                    .filter(|f| f.path == current_path_str)
                    .max_by_key(|f| f.modify_time);

                match latest_file_record {
                    Some(existing_file) => {
                        let size = fs::metadata(&current_path)?.len();
                        let hash = util::hash::calculate_file_hash(&current_path)?;

                        if existing_file.deleted {
                            println!("File restored: {}", current_path_str);
                            file_info_new.push(FileInfo::new_simple(
                                current_path_str,
                                size,
                                hash,
                                chrono::Utc::now(),
                                false,
                            ));
                        }
                        // Якщо файл існував і змінився
                        else if existing_file.size != size || existing_file.hash != hash {
                            println!("File changed: {}", current_path_str);
                            file_info_new.push(FileInfo::new_simple(
                                current_path_str,
                                size,
                                hash,
                                chrono::Utc::now(),
                                false,
                            ));
                        } else {
                            println!("File unchanged: {}", current_path_str);
                        }
                    }
                    None => {
                        println!("New file: {}", current_path_str);
                        let size = fs::metadata(&current_path)?.len();
                        let hash = util::hash::calculate_file_hash(&current_path)?;

                        file_info_new.push(FileInfo::new_simple(
                            current_path_str,
                            size,
                            hash,
                            chrono::Utc::now(),
                            false,
                        ));
                    }
                }
            }
        }

        // Групуємо файли по шляху і беремо тільки найновіші записи
        let mut latest_files: std::collections::HashMap<String, FileInfo> =
            std::collections::HashMap::new();
        for file_info in file_infos {
            match latest_files.get(&file_info.path) {
                Some(existing) => {
                    // Якщо поточний файл новіший, замінюємо
                    if file_info.modify_time > existing.modify_time {
                        latest_files.insert(file_info.path.clone(), file_info);
                    }
                }
                None => {
                    latest_files.insert(file_info.path.clone(), file_info);
                }
            }
        }

        // Додаємо видалені файли (тільки ті що не були видалені раніше)
        for (path, latest_file) in latest_files {
            if !processed_paths.contains(&path) && (!latest_file.deleted) {
                println!("File deleted: {}", path);
                let mut deleted_file = latest_file;
                deleted_file.deleted = true;
                deleted_file.modify_time = chrono::Utc::now();
                file_info_new.push(deleted_file);
            }
        }

        println!("Total changes to backup: {} files", file_info_new.len());
        
        // Зберігаємо контент для кожного файлу і повертаємо оновлений список
        let mut updated_file_infos = Vec::new();
        for mut file_info in file_info_new {
            if !file_info.deleted {
                // Зберігаємо контент тільки для не видалених файлів
                match FileInfo::store_content(
                    &file_info.path,
                    &file_info.hash,
                    &PathBuf::from(&backup_path),
                    ContentType::FullCopy,
                ) {
                    Ok(content_path) => {
                        file_info.content_path = Some(content_path);
                        file_info.content_type = ContentType::FullCopy;
                        println!("Stored content for: {}", file_info.path);
                    }
                    Err(e) => {
                        println!("Failed to store content for {}: {}", file_info.path, e);
                        // Все одно додаємо файл, але без збереженого контенту
                        file_info.content_type = ContentType::Unchanged;
                    }
                }
            } else {
                // Для видалених файлів контент не потрібен
                file_info.content_type = ContentType::Unchanged;
                file_info.content_path = None;
            }
            updated_file_infos.push(file_info);
        }
        
        Ok(updated_file_infos)
    }

    pub(crate) fn restore(backup_number: u32, path: &PathBuf) -> anyhow::Result<()> {
        let config = Config::read_config();
        let backup_path = config.get_default_backup_path();
        let backup_info_path = config.get_default_backup_info_path();
        let existing_backups = BackupInfo::get_backup_info_by_path(backup_info_path);

        // Find backup for the given path
        let backup_info = existing_backups
            .into_iter()
            .find(|backup| backup.0.path_to_root == *path)
            .ok_or(anyhow::anyhow!(
                "No backup found for path: {}",
                path.display()
            ))?
            .0;

        // Get all backup files up to the specified number inclusive
        let backup_files_paths = Self::get_backup_files_by_prefix(
            &PathBuf::from(backup_path),
            &backup_info.backup_prefix,
        );
        let backup_file_paths: Vec<_> = backup_files_paths
            .iter()
            .filter_map(|path| {
                let file_name = path.file_name()?.to_string_lossy();
                if file_name.starts_with("backup_") && file_name.ends_with(".json") {
                    let number_str = file_name
                        .trim_start_matches("backup_")
                        .trim_end_matches(".json");
                    let number = number_str.parse::<u32>().ok()?;
                    if number <= backup_number {
                        Some((number, path.clone()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        if backup_file_paths.is_empty() {
            return Err(anyhow::anyhow!(
                "No backup files found up to backup #{}",
                backup_number
            ));
        }

        // Read files from all backups up to specified number
        let mut all_file_infos = Vec::new();
        for (_, backup_path) in &backup_file_paths {
            let files = FileInfo::get_file_info_by_path(backup_path);
            all_file_infos.extend(files);
        }

        // Групуємо файли по шляху і беремо тільки найновіші записи
        let mut latest_files: std::collections::HashMap<String, FileInfo> =
            std::collections::HashMap::new();
        for file_info in all_file_infos {
            match latest_files.get(&file_info.path) {
                Some(existing) => {
                    // Якщо поточний файл новіший, замінюємо
                    if file_info.modify_time > existing.modify_time {
                        latest_files.insert(file_info.path.clone(), file_info);
                    }
                }
                None => {
                    latest_files.insert(file_info.path.clone(), file_info);
                }
            }
        }

        // Фільтруємо тільки файли які не видалені
        let file_infos: Vec<_> = latest_files.into_values().filter(|f| !f.deleted).collect();

        if file_infos.is_empty() {
            println!("No files found in backup #{}", backup_number);
            return Ok(());
        }

        println!(
            "Restoring {} files from backup #{}...",
            file_infos.len(),
            backup_number
        );

        // Відновлюємо файли
        for file_info in file_infos {
            if !file_info.deleted {
                match Self::restore_single_file(&file_info) {
                    Ok(_) => println!("✓ Restored: {}", file_info.path),
                    Err(e) => println!("✗ Failed to restore {}: {}", file_info.path, e),
                }
            }
        }

        println!("Restore completed!");
        Ok(())
    }

    fn restore_single_file(file_info: &FileInfo) -> anyhow::Result<()> {
        let file_path = Path::new(&file_info.path);
        println!("restore file: {}", file_path.display());
        let restore_path = file_path;

        let config = Config::read_config();
        let backup_path = config.get_default_backup_path();
        let backup_dir = PathBuf::from(backup_path);

        FileInfo::restore_content(file_info, &backup_dir, &restore_path.to_path_buf())?;

        Ok(())
    }

    pub(crate) fn list_backups(path: &PathBuf) -> anyhow::Result<()> {
        let config = Config::read_config();
        let backup_path = config.get_default_backup_path();
        let backup_info_path = config.get_default_backup_info_path();
        let existing_backups = BackupInfo::get_backup_info_by_path(backup_info_path);

        // Find backup for the given path
        let backup_info = existing_backups
            .into_iter()
            .find(|backup| backup.0.path_to_root == *path)
            .ok_or(anyhow::anyhow!(
                "No backup found for path: {}",
                path.display()
            ))?
            .0;

        // Отримуємо всі backup файли
        let backup_files = Self::get_backup_files_by_prefix(
            &PathBuf::from(backup_path),
            &backup_info.backup_prefix,
        );

        if backup_files.is_empty() {
            println!("No backup files found for this project");
            return Ok(());
        }

        println!("Available backups for: {}", path.display());
        println!("Backup prefix: {}", backup_info.backup_prefix);
        println!("─────────────────────────────────────────────");

        // Сортуємо backup файли по номеру
        let mut numbered_backups: Vec<(u32, PathBuf)> = backup_files
            .into_iter()
            .filter_map(|path| {
                let file_name = path.file_name()?.to_string_lossy();
                if file_name.starts_with("backup_") && file_name.ends_with(".json") {
                    let number_str = file_name
                        .trim_start_matches("backup_")
                        .trim_end_matches(".json");
                    let number = number_str.parse::<u32>().ok()?;
                    Some((number, path))
                } else {
                    None
                }
            })
            .collect();

        numbered_backups.sort_by_key(|(number, _)| *number);

        for (number, backup_path) in numbered_backups {
            let file_infos = FileInfo::get_file_info_by_path(&backup_path);
            let changes = file_infos.iter().filter(|f| !f.deleted).count();
            let deletions = file_infos.iter().filter(|f| f.deleted).count();

            if let Ok(metadata) = fs::metadata(&backup_path) {
                if let Ok(modified) = metadata.modified() {
                    let datetime: chrono::DateTime<chrono::Utc> = modified.into();
                    println!(
                        "Backup #{}: {} changes, {} deletions ({})",
                        number,
                        changes,
                        deletions,
                        datetime.format("%Y-%m-%d %H:%M:%S UTC")
                    );
                }
            }
        }

        println!("\nUse: snapback restore <backup_number> <path>");
        Ok(())
    }
}

fn generate_prefix(root_dir: &PathBuf) -> String {
    let config = Config::read_config();
    let backup_info_path = config.get_default_backup_info_path();
    let existing_backups = BackupInfo::get_backup_info_by_path(backup_info_path);

    for (backup_info, _) in existing_backups {
        if backup_info.path_to_root == *root_dir {
            return backup_info.backup_prefix;
        }
    }

    let root_dir_str = root_dir.to_string_lossy().to_string();
    let uuid = Uuid::new_v4().to_string();
    let mut prefix = root_dir_str.split("/").last().unwrap().to_string();
    prefix.push_str(&uuid);
    prefix
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct FileInfo {
    path: String,
    size: u64,
    hash: String,
    modify_time: DateTime<chrono::Utc>,
    deleted: bool,
    content_type: ContentType,
    content_path: Option<String>, // Шлях до збереженого контенту
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
enum ContentType {
    FullCopy,
    Delta { base_hash: String },
    Unchanged,
}

impl FileInfo {
    fn new(
        path: String,
        size: u64,
        hash: String,
        modify_time: DateTime<chrono::Utc>,
        deleted: bool,
        content_type: ContentType,
        content_path: Option<String>,
    ) -> Self {
        Self {
            path,
            size,
            hash,
            modify_time,
            deleted,
            content_type,
            content_path,
        }
    }

    fn new_simple(
        path: String,
        size: u64,
        hash: String,
        modify_time: DateTime<chrono::Utc>,
        deleted: bool,
    ) -> Self {
        Self::new(
            path,
            size,
            hash,
            modify_time,
            deleted,
            ContentType::Unchanged,
            None,
        )
    }

    fn get_file_info_by_path(path: &PathBuf) -> Vec<FileInfo> {
        let content = fs::read_to_string(path).ok().unwrap_or_default();
        serde_json::from_str::<Vec<FileInfo>>(&content).unwrap_or_default()
    }

    fn get_vec_file_info_by_paths(paths: Vec<PathBuf>) -> Vec<FileInfo> {
        paths
            .iter()
            .flat_map(|path| Self::get_file_info_by_path(path))
            .collect()
    }

    fn store_content(
        file_path: &str,
        content_hash: &str,
        backup_dir: &PathBuf,
        content_type: ContentType,
    ) -> anyhow::Result<String> {
        let content_dir = backup_dir.join("content");
        fs::create_dir_all(&content_dir)?;

        let content_file_path = content_dir.join(format!("{}.dat", content_hash));
        let relative_path = content_file_path
            .strip_prefix(backup_dir)
            .unwrap_or(&content_file_path)
            .to_string_lossy()
            .to_string();

        match content_type {
            ContentType::FullCopy => {
                fs::copy(file_path, &content_file_path)?;
                println!("Stored full copy: {}", file_path);
            }
            ContentType::Delta { .. } => {
                fs::copy(file_path, &content_file_path)?;
                println!("Stored delta (full for now): {}", file_path);
            }
            ContentType::Unchanged => {
                return Ok(String::new());
            }
        }

        Ok(relative_path)
    }

    /// Відновлює контент файлу з backup'а
    fn restore_content(
        file_info: &FileInfo,
        backup_dir: &PathBuf,
        target_path: &PathBuf,
    ) -> anyhow::Result<()> {
        if let Some(content_path) = &file_info.content_path {
            if !content_path.is_empty() {
                let source_path = backup_dir.join(content_path);

                if source_path.exists() {
                    if let Some(parent) = target_path.parent() {
                        fs::create_dir_all(parent)?;
                    }

                    fs::copy(&source_path, target_path)?;
                    println!("Restored content from: {}", source_path.display());
                    return Ok(());
                }
            }
        }

        // Fallback: створюємо порожній файл
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target_path, vec![0u8; file_info.size as usize])?;
        println!(
            "Created placeholder file (no content stored): {}",
            target_path.display()
        );

        Ok(())
    }
}
