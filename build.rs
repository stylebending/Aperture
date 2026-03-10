use image::{ImageFormat, ImageReader};
use std::io;
use std::path::Path;

fn main() -> io::Result<()> {
    // Convert PNG to ICO if it doesn't exist
    let icon_path = Path::new("assets/icon.ico");
    if !icon_path.exists() {
        println!("cargo:warning=Converting ApertureLogo.png to assets/icon.ico");

        let img = ImageReader::open("ApertureLogo.png")
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
            .decode()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // Resize to 256x256 (max size for ICO format)
        let resized = img.resize(256, 256, image::imageops::FilterType::Lanczos3);

        resized
            .save_with_format(icon_path, ImageFormat::Ico)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    }

    // Compile the Windows resource file with the icon
    winres::WindowsResource::new()
        .set_icon("assets/icon.ico")
        .compile()?;

    Ok(())
}
