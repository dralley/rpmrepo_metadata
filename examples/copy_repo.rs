// Read an existing RPM repository and write it out with different options.
//
// This demonstrates round-tripping repository metadata through the high-level
// Repository API, useful for re-compressing metadata, changing checksum types,
// or normalizing a repository's layout.
//
// Usage: cargo run --example copy_repo -- <input_repo> <output_dir> [--zstd|--xz|--gz|--bz2] [--simple-filenames]

use std::path::Path;

use rpmrepo_metadata::{CompressionType, Repository, RepositoryOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "usage: copy_repo <input_repo> <output_dir> [--zstd|--xz|--gz|--bz2] [--simple-filenames]"
        );
        std::process::exit(1);
    }

    let input_path = Path::new(&args[1]);
    let output_path = Path::new(&args[2]);

    let mut compression = CompressionType::Zstd;
    let mut simple_filenames = false;

    for arg in &args[3..] {
        match arg.as_str() {
            "--zstd" => compression = CompressionType::Zstd,
            "--xz" => compression = CompressionType::Xz,
            "--gz" => compression = CompressionType::Gzip,
            "--bz2" => compression = CompressionType::Bz2,
            "--simple-filenames" => simple_filenames = true,
            other => {
                eprintln!("Unknown option: {other}");
                std::process::exit(1);
            }
        }
    }

    println!("Reading repository from: {}", input_path.display());
    let mut repo = Repository::load_from_directory(input_path)?;

    println!("  {} packages", repo.packages().len());
    println!("  {} advisories", repo.advisories().len());
    println!("  {} groups", repo.groups().len());
    println!("  {} categories", repo.categories().len());
    println!("  {} environments", repo.environments().len());

    // Sort packages for better compression ratios
    repo.sort();

    let options = RepositoryOptions::default()
        .compression_type(compression)
        .simple_metadata_filenames(simple_filenames);

    println!(
        "\nWriting repository to: {} (compression: {:?}, simple filenames: {})",
        output_path.display(),
        compression,
        simple_filenames
    );
    repo.write_to_directory_with_options(output_path, options)?;

    // Verify by reading back
    let verify = Repository::load_from_directory(output_path)?;
    assert_eq!(
        repo.packages().len(),
        verify.packages().len(),
        "Package count mismatch after round-trip!"
    );
    println!(
        "\nVerified: {} packages written and read back successfully.",
        verify.packages().len()
    );

    Ok(())
}
