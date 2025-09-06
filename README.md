# SnapBack

A fast, intelligent backup tool written in Rust that tracks file changes with content-aware storage.

## Features

- **🚀 Fast Performance**: Efficient file scanning and hashing using Rust
- **📦 Smart Storage**: Only stores changed files, not duplicates
- **🔄 Point-in-Time Restore**: Restore to any backup state
- **💾 Content Deduplication**: Files with same content stored once
- **📊 Change Tracking**: Track new, modified, and deleted files
- **🎯 Incremental Backups**: Only backup what has changed
- **📋 Backup Listing**: View all available backups with statistics

## Installation

### Prerequisites
- Rust 1.70+ installed
- Git (for cloning)

### Build from Source
```bash
git clone https://github.com/Knotty123230/snapback.git
cd snapback
cargo build --release
```

The binary will be available at `target/release/snapback`

## Usage

### Create a Backup
```bash
snapback create /path/to/your/project
```

This creates an incremental backup of all files in the specified directory.

### List Available Backups
```bash
snapback list /path/to/your/project
```

Output example:
```
Available backups for: /path/to/your/project
Backup prefix: project_name_abc123def456
─────────────────────────────────────────────
Backup #0: 1250 changes, 0 deletions (2024-01-15 14:30:22 UTC)
Backup #1: 5 changes, 2 deletions (2024-01-15 15:45:33 UTC)
Backup #2: 0 changes, 1 deletions (2024-01-15 16:12:44 UTC)

Use: snapback restore <backup_number> <path>
```

### Restore from Backup
```bash
snapback restore 1 /path/to/your/project
```

This restores all files to their state at backup #1.

### Configuration Management
```bash
# View current configuration
snapback config show

# Initialize default config file
snapback config init

# Set custom paths
snapback config path --backup-path ~/my-backups --info-path ~/my-backup-info
```

### Advanced Usage with Environment Variables
```bash
# Set custom backup location for session
export SNAPBACK_BACKUP_PATH="/external/drive/backups"

# Create backup with custom settings
snapback create ~/important-project

# Show where files are being stored
snapback config show
```

## How It Works

### Intelligent Change Detection
SnapBack uses SHA-256 hashing to detect file changes efficiently:
- **New files**: Added to backup with full content
- **Modified files**: Only changed files stored 
- **Deleted files**: Marked as deleted, no content stored
- **Unchanged files**: Not stored again (deduplication)

### Storage Structure
```
backups/
├── project_name_uuid/
│   ├── backup_0.json     # First backup metadata
│   ├── backup_1.json     # Incremental changes
│   └── content/          # Actual file contents
│       ├── hash1.dat     # File content by hash
│       └── hash2.dat
└── backup_info/
    └── project_name_uuid.json  # Project metadata
```

### Point-in-Time Restore
When restoring backup #N, SnapBack:
1. Reads all backups from #0 to #N
2. Builds final state by applying changes chronologically  
3. Restores latest version of each non-deleted file
4. Skips files that were deleted before backup #N

## Configuration

SnapBack uses a flexible multi-level configuration system with the following priority order:

1. **Environment Variables** (highest priority)
2. **User Config Directory** (`~/.config/snapback/config.json` on Linux/macOS)  
3. **Local Project Config** (`./snapback.json` in current directory)
4. **Default Values** (fallback)

### Configuration File

SnapBack automatically creates a configuration file with these settings:

```json
{
  "backup_default_path": null,
  "backup_info_default_path": null,
  "max_backup_count": 100,
  "compress_backups": false,
  "exclude_patterns": [
    "target/", 
    "node_modules/", 
    ".git/", 
    "*.tmp", 
    "*.log"
  ]
}
```

### Configuration Commands

```bash
# Show current configuration
snapback config show

# Initialize configuration file
snapback config init

# Set custom backup paths
snapback config path --backup-path /custom/backups --info-path /custom/info
```

### Environment Variables

Override configuration with environment variables:

```bash
# Set backup directories
export SNAPBACK_BACKUP_PATH="/path/to/backups"
export SNAPBACK_INFO_PATH="/path/to/backup_info"

# Set backup limits and options
export SNAPBACK_MAX_BACKUPS=50
export SNAPBACK_COMPRESS=true
```

### Default Paths

SnapBack uses platform-appropriate default paths:

- **macOS**: `~/Library/Application Support/snapback/`
- **Linux**: `~/.local/share/snapback/`  
- **Windows**: `%APPDATA%\snapback\`
- **Fallback**: Current directory

### Project-Specific Config

Create a `snapback.json` file in your project root for project-specific settings:

```json
{
  "exclude_patterns": [
    "build/",
    "dist/", 
    "*.cache"
  ],
  "max_backup_count": 20
}
```

## Examples

### Daily Development Backup
```bash
# Start of day
snapback create ~/my-project

# After making changes
snapback create ~/my-project  # Only changed files backed up

# View history
snapback list ~/my-project

# Restore to earlier state
snapback restore 0 ~/my-project
```

### Project Versioning
```bash
# Version 1.0 release
snapback create ~/my-app
echo "v1.0" > version.txt

# Version 1.1 development
# ... make changes ...
snapback create ~/my-app

# Rollback to v1.0 if needed
snapback restore 0 ~/my-app
```

## Performance

SnapBack is designed for speed:
- **Parallel file processing**: Uses Rust's async capabilities
- **Smart hashing**: Only rehashes files with changed metadata
- **Efficient storage**: Content deduplication saves space
- **Fast restore**: Direct file copying, no complex reconstruction

### Benchmarks (typical 1000-file project)
- Initial backup: ~2-5 seconds
- Incremental backup (10 changed files): ~0.5-1 second  
- List backups: ~0.1 seconds
- Restore: ~1-3 seconds

## File Handling

### Supported File Types
- All text files (source code, documents, configs)
- Binary files (images, executables, archives)
- Large files (handled efficiently)
- Symlinks (preserved as-is)

### Ignored Files
Currently backs up all files. Future versions may add:
- `.snapbackignore` support
- Git-style ignore patterns
- Size limits

## Error Handling

SnapBack handles common issues gracefully:
- **Permission errors**: Skipped with warning
- **Missing directories**: Created automatically
- **Corrupted backups**: Detailed error messages
- **Disk space**: Checked before operations

## Contributing

We welcome contributions! Please:

1. Fork the repository
2. Create a feature branch: `git checkout -b feature-name`
3. Make changes with tests
4. Submit a pull request

### Development Setup
```bash
git clone https://github.com/Knotty123230/snapback.git
cd snapback
cargo test
cargo clippy
```

## Roadmap

### Completed ✅
- [x] **Multi-level configuration** system
- [x] **Platform-specific default paths** 
- [x] **Environment variable support**
- [x] **File exclusion patterns**
- [x] **Project-specific configuration**

### Planned 🔄
- [ ] **Delta compression** for text files
- [ ] **Advanced ignore patterns** (glob support)
- [ ] **Encryption** for sensitive backups  
- [ ] **Remote storage** backends (S3, etc.)
- [ ] **Automatic cleanup** of old backups
- [ ] **Progress indicators** for large backups
- [ ] **GUI interface**
- [ ] **Scheduled backups**
- [ ] **Backup verification** and repair

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Changelog

### v0.1.0 (Current)
- ✅ Basic backup and restore functionality
- ✅ Incremental backups with change detection
- ✅ Content deduplication
- ✅ Point-in-time restore
- ✅ Backup listing and statistics
- ✅ Cross-platform support (Windows, macOS, Linux)

## FAQ

**Q: How much disk space do backups use?**
A: Only changed files are stored. For typical code projects, expect 10-50MB for initial backup, then 1-10MB per incremental backup.

**Q: Can I backup the same project from multiple locations?**
A: Yes! SnapBack uses project paths to identify unique backup sets.

**Q: What happens if backup is interrupted?**
A: The backup process is atomic - either completes fully or leaves previous state intact.

**Q: How do I delete old backups?**
A: Currently manual - delete backup files from the backup directory. Automatic cleanup is planned.

---

Built with ❤️ in Rust for developers who value their code.