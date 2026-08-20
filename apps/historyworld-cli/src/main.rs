use std::{env, error::Error, fs::File, io::Read, path::PathBuf, process::ExitCode, str::FromStr};

use historyworld_content::CompiledRelease;
use historyworld_domain::{DetailLevel, LayerMask, ResourceBudget, WorldCell, WorldInstant};
use historyworld_sim::{Materializer, SnapshotRequest};
use historyworld_storage::{FilesystemObjectStore, ObjectStore};

const DEMO_RELEASE: &str = include_str!("../../../data/fixtures/vesuvius-architecture-demo.json");
const DEMO_CELL: &str = "0:14:ee085cd";
const MAX_RELEASE_SOURCE_BYTES: u64 = 16 * 1024 * 1024;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args().skip(1);
    match arguments.next().as_deref() {
        Some("validate") => {
            let release = load_release(arguments.next().as_deref())?;
            println!(
                "valid release {}: {} patches across {} indexed cells",
                release.manifest().release_id,
                release.manifest().patch_count,
                release.manifest().indexed_cell_count
            );
        }
        Some("manifest") => {
            let release = load_release(arguments.next().as_deref())?;
            println!("{}", serde_json::to_string_pretty(release.manifest())?);
        }
        Some("snapshot") => snapshot(&arguments.collect::<Vec<_>>())?,
        Some("publish") => publish(&arguments.collect::<Vec<_>>())?,
        Some("verify-object") => verify_object(&arguments.collect::<Vec<_>>())?,
        Some("help" | "--help" | "-h") | None => print_help(),
        Some(command) => return Err(format!("unknown command '{command}'; use --help").into()),
    }
    Ok(())
}

fn load_release(path: Option<&str>) -> Result<CompiledRelease, Box<dyn Error>> {
    Ok(CompiledRelease::from_json(&release_json(path)?)?)
}

fn release_json(path: Option<&str>) -> Result<String, Box<dyn Error>> {
    let Some(path) = path else {
        return Ok(DEMO_RELEASE.to_owned());
    };
    let mut source = String::new();
    File::open(path)?
        .take(MAX_RELEASE_SOURCE_BYTES.saturating_add(1))
        .read_to_string(&mut source)?;
    let actual = u64::try_from(source.len()).unwrap_or(u64::MAX);
    if actual > MAX_RELEASE_SOURCE_BYTES {
        return Err(format!(
            "release source is {actual} bytes, above the {MAX_RELEASE_SOURCE_BYTES}-byte compiler input limit"
        )
        .into());
    }
    Ok(source)
}

fn snapshot(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    let mut year = 80_i64;
    let mut month = 1_u8;
    let mut day = 1_u8;
    let mut cell = WorldCell::from_str(DEMO_CELL)?;
    let mut detail = DetailLevel::Local;
    let mut budget = ResourceBudget::INTERACTIVE;
    let mut input = None::<String>;

    let mut index = 0;
    while index < arguments.len() {
        let option = arguments[index].as_str();
        index += 1;
        let value = arguments
            .get(index)
            .ok_or_else(|| format!("missing value after {option}"))?;
        index += 1;
        match option {
            "--at-year" => year = value.parse()?,
            "--month" => month = value.parse()?,
            "--day" => day = value.parse()?,
            "--cell" => cell = value.parse()?,
            "--detail" => detail = parse_detail(value)?,
            "--max-instances" => budget.max_generated_instances = value.parse()?,
            "--input" => input = Some(value.clone()),
            _ => return Err(format!("unknown snapshot option '{option}'").into()),
        }
    }

    budget = ResourceBudget::new(
        budget.max_candidate_patches,
        budget.max_output_bytes,
        budget.max_generated_instances,
        budget.max_work_units,
    )?;

    let release = load_release(input.as_deref())?;
    let materializer = Materializer::new(release);
    let result = materializer.snapshot(SnapshotRequest {
        cell,
        at: WorldInstant::from_ymd_hms(year, month, day, 0, 0, 0)?,
        layers: LayerMask::ALL,
        requested_detail: detail,
        budget,
    })?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

fn parse_detail(value: &str) -> Result<DetailLevel, Box<dyn Error>> {
    match value {
        "planet" => Ok(DetailLevel::Planet),
        "continental" => Ok(DetailLevel::Continental),
        "regional" => Ok(DetailLevel::Regional),
        "local" => Ok(DetailLevel::Local),
        "street" => Ok(DetailLevel::Street),
        _ => Err(format!("invalid detail '{value}'").into()),
    }
}

fn publish(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    let mut data_dir = PathBuf::from("artifacts");
    let mut input = None::<String>;
    parse_pairs(arguments, |option, value| match option {
        "--data-dir" => {
            data_dir = PathBuf::from(value);
            Ok(())
        }
        "--input" => {
            input = Some(value.to_owned());
            Ok(())
        }
        _ => Err(format!("unknown publish option '{option}'").into()),
    })?;

    let source_json = release_json(input.as_deref())?;
    let release = CompiledRelease::from_json(&source_json)?;
    let store = FilesystemObjectStore::open(&data_dir, 256 * 1024 * 1024)?;
    let source_object = store.put(source_json.as_bytes())?;
    let manifest_bytes = serde_json::to_vec(release.manifest())?;
    let manifest_object = store.put(&manifest_bytes)?;
    let descriptor = serde_json::to_vec(&serde_json::json!({
        "contract": "historyworld.demo-published-release.v1",
        "release_id": release.manifest().release_id,
        "source_document_object": source_object.hash,
        "manifest_object": manifest_object.hash
    }))?;
    let descriptor_object = store.put(&descriptor)?;
    println!(
        "published DEMO release {} (dependency closure is not production-verified)\nmanifest object {}\nsource object {}\ndescriptor object {}",
        release.manifest().release_id,
        manifest_object.hash,
        source_object.hash,
        descriptor_object.hash
    );
    Ok(())
}

fn verify_object(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    let mut data_dir = PathBuf::from("artifacts");
    let mut hash = None;
    parse_pairs(arguments, |option, value| match option {
        "--data-dir" => {
            data_dir = PathBuf::from(value);
            Ok(())
        }
        "--hash" => {
            hash = Some(value.parse()?);
            Ok(())
        }
        _ => Err(format!("unknown verify-object option '{option}'").into()),
    })?;
    let hash = hash.ok_or("verify-object requires --hash")?;
    let store = FilesystemObjectStore::open(&data_dir, 256 * 1024 * 1024)?;
    let bytes = store.verify(hash)?;
    println!("verified object {hash}: {bytes} bytes");
    Ok(())
}

fn parse_pairs(
    arguments: &[String],
    mut apply: impl FnMut(&str, &str) -> Result<(), Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    let mut pairs = arguments.chunks_exact(2);
    for pair in &mut pairs {
        apply(&pair[0], &pair[1])?;
    }
    if let Some(option) = pairs.remainder().first() {
        return Err(format!("missing value after {option}").into());
    }
    Ok(())
}

fn print_help() {
    println!(
        "HistoryWorld foundation CLI\n\n\
         Commands:\n\
           validate [release.json]\n\
           manifest [release.json]\n\
           snapshot [--at-year YEAR] [--month M] [--day D] [--cell F:L:PATH]\n\
                    [--detail planet|continental|regional|local|street]\n\
                    [--max-instances N] [--input release.json]\n\
           publish [--data-dir DIR] [--input release.json]  # demo/local; no dependency closure\n\
           verify-object [--data-dir DIR] --hash BLAKE3\n\n\
         The bundled Vesuvius fixture is illustrative architecture data, not a historical source."
    );
}
