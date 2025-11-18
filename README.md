# blakediff 🐿️

A fast, parallel file hashing and comparison tool using the BLAKE3 algorithm.

**Features:**
- 🚀 Fast BLAKE3 hashing with parallel processing
- 📊 Multiple output formats (Text, JSON, CSV)
- 🔍 Duplicate file detection
- ⚖️ File set comparison between directories
- 🧵 Optional parallel directory traversal
- ✅ Comprehensive test coverage
- 🛡️ Robust error handling

## Installation

### Quick install
```bash
cargo install --path .
```

### Update to latest version
```bash
git pull
cargo install --path .
```

## Building from source

```bash
# Build in debug mode
cargo build

# Build optimized release
cargo build --release

# Run tests
cargo test

# Run linters
cargo clippy
cargo fmt --check
```


## Usage

### Command: `generate`

Generate a hash report for all files in a directory.

**Basic usage:**
```bash
blakediff generate <directory> > report.txt
```

**Options:**
- `--parallel, -p`: Use parallel directory traversal (recommended for SSDs only)

**Output format:**
```
<hash> <file_path>
```

**Examples:**
```bash
# Generate hash report for local music directory
blakediff generate ~/Music > ~/music_local.txt

# Generate with parallel processing
blakediff generate --parallel ~/Music > ~/music_local.txt

# Generate hash report for network mount
blakediff generate /mnt/smbmount/Music > ~/music_smb.txt
```

---

### Command: `analyze`

Find duplicate files within a single report.

**Basic usage:**
```bash
blakediff analyze <report_file>
```

**Options:**
- `--format, -f`: Output format (`text`, `json`, `csv`) - default: `text`

**Examples:**
```bash
# Find duplicates (text output)
blakediff analyze ~/music_local.txt

# Find duplicates (JSON output)
blakediff analyze --format json ~/music_local.txt

# Find duplicates (CSV output)
blakediff analyze --format csv ~/music_local.txt
```

**Text output example:**
```
duplicates : /home/user/Music/song1.mp3 🟰 /home/user/Music/copy/song1.mp3
duplicates : /home/user/Music/song2.mp3 🟰 /home/user/Music/backup/song2.mp3
```

---

### Command: `compare`

Compare two hash reports and show unique/duplicate files.

**Basic usage:**
```bash
blakediff compare <report_1> <report_2>
```

**Options:**
- `--format, -f`: Output format (`text`, `json`, `csv`) - default: `text`

**Examples:**
```bash
# Compare local and network music directories
blakediff compare ~/music_local.txt ~/music_smb.txt

# Compare with JSON output
blakediff compare --format json ~/music_local.txt ~/music_smb.txt

# Compare with CSV output
blakediff compare --format csv ~/music_local.txt ~/music_smb.txt
```

**Text output example:**
```
only in ~/music_local.txt : /home/user/Music/new_song.mp3
only in ~/music_smb.txt : /mnt/smbmount/Music/old_song.mp3
duplicates : /home/user/Music/shared.mp3 🟰 /mnt/smbmount/Music/shared.mp3
```


## Performance

### Why BLAKE3?

BLAKE3 is significantly faster than SHA256 while maintaining excellent cryptographic properties:
- Parallel processing support
- Optimized for modern CPUs with SIMD instructions
- Ideal for file hashing and integrity verification

### Benchmarking

To benchmark without I/O bottlenecks, use a tmpfs ramdisk:

```bash
# Create 10GB ramdisk
sudo mount -t tmpfs -o size=10G tmpfs /media/ramdisk

# Copy test files
cp -r ~/test_directory /media/ramdisk/

# Benchmark SHA256
time (find /media/ramdisk -type f -exec sha256sum {} \;)

# Benchmark BLAKE3 (blakediff)
time blakediff generate /media/ramdisk

# Cleanup
sudo umount /media/ramdisk
```

### Performance tips

- Use `--parallel` flag for SSDs (not recommended for HDDs)
- Files > 16KB are automatically memory-mapped for faster hashing
- Parallel hashing is automatically used for memory-mapped files

---

## Development

### Running tests

```bash
# Run all tests (unit + integration)
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_analyze_command
```

### Code quality

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --check

# Run clippy lints
cargo clippy --all-targets --all-features

# Run clippy with warnings as errors
cargo clippy --all-targets --all-features -- -D warnings
```

### CI/CD

The project uses GitHub Actions for continuous integration:
- ✅ Build verification
- ✅ Test execution (unit + integration)
- ✅ Code formatting check (rustfmt)
- ✅ Linting (clippy)

---

## Technical Details

### Hash Report Format

Each line in the report follows this format:
```
<blake3_hash> <absolute_file_path>
```

Example:
```
af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262 /home/user/file.txt
```

### Architecture

- **Input module**: Optimized file reading with automatic mmap selection
- **Parallel processing**: Rayon for multi-threaded hashing
- **Memory efficiency**: Smart buffer sizing (64KB for optimal SIMD performance)
- **Error handling**: Comprehensive error propagation with helpful messages

---

## License

MIT License - See LICENSE file for details

## Contributing

Contributions welcome! Please ensure:
1. All tests pass (`cargo test`)
2. Code is formatted (`cargo fmt`)
3. No clippy warnings (`cargo clippy`)
4. Add tests for new features