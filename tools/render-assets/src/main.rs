use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use asset_config::AssetConfig;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use structopt::StructOpt;

const RENDER_ASSET_SCRIPT: &str = include_str!(concat!(
	env!("CARGO_MANIFEST_DIR"),
	"/scripts/render-asset.py"
));

#[derive(Debug, Copy, Clone, Hash)]
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Opts {
	#[structopt(long)]
	no_override: bool,
}

#[cfg(target_family = "windows")]
fn blender_exe() -> PathBuf {
	PathBuf::from("C:")
		.join("Program Files")
		.join("Blender Foundation")
		.join("Blender 3.0")
		.join("blender.exe")
}

#[cfg(not(target_family = "windows"))]
fn blender_exe() -> PathBuf {
	PathBuf::from("blender")
}

fn main() {
	let opts = Opts::from_args();

	let out_dir = PathBuf::from("assets").join("rendered");

	let render_config_path = PathBuf::from("render_assets.toml");
	let render_config_dir = render_config_path.parent().unwrap();

	let render_config_str = fs::read_to_string(&render_config_path).unwrap();
	let render_config: AssetConfig = toml::from_str(&render_config_str).unwrap();

	let progress =
		ProgressBar::new(render_config.file.values().flat_map(|v| v.iter()).count() as u64);
	progress.set_style(
		ProgressStyle::with_template("{spinner:.green} {msg} [{wide_bar}] {pos}/{len} {percent}%")
			.unwrap()
			.progress_chars("=> "),
	);
	progress.enable_steady_tick(Duration::from_millis(200));
	progress.inc(0);

	for (blend_file_name, assets_config) in &render_config.file {
		let blend_file_path = render_config_dir.join(&blend_file_name);
		for (asset_name, asset_config) in assets_config {
			let out_filename = render_config.get_asset_output(asset_name).unwrap();
			let out_path = out_dir.join(&out_filename);

			progress.set_message(format!(
				"Rendering {} | {} > {}",
				blend_file_path.file_name().unwrap().to_string_lossy(),
				&asset_config.object,
				out_filename.file_name().unwrap().to_string_lossy()
			));

			if !(opts.no_override && out_path.exists()) {
				let mut blender_cmd = Command::new(blender_exe());
				blender_cmd
					.arg("--background")
					.arg(&blend_file_path)
					.arg("--python-expr")
					.arg(RENDER_ASSET_SCRIPT)
					.arg("--")
					.arg("--output")
					.arg(out_path)
					.arg("--object-name")
					.arg(&asset_config.object)
					.arg("--width")
					.arg(asset_config.width.to_string())
					.arg("--z-local-frames")
					.arg(asset_config.z_local_frames.to_string())
					.arg("--z-frames")
					.arg(asset_config.z_frames.to_string())
					.arg("--x-frames")
					.arg(asset_config.x_frames.to_string());

				let blender_out = blender_cmd.output().unwrap_or_else(|err| {
					panic!("Failed to render {}: {err}", &asset_config.object)
				});

				eprintln!("-- blender stdout:");
				eprintln!("{}", String::from_utf8_lossy(&blender_out.stdout));
				eprintln!("-- blender stderr:");
				eprintln!("{}", String::from_utf8_lossy(&blender_out.stderr));

				if !blender_out.status.success() {
					panic!("Failed to render {}:", &asset_config.object)
				}
			}

			progress.inc(1);
		}
	}
}
