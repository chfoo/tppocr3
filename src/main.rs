use clap::{App, Arg};
use tppocr::{
    config::ProcessorConfig, frame::FrameReader, processor::Processor,
    text_recognizer::TextRecognizer, vnc::VncClient,
};

fn main() -> anyhow::Result<()> {
    tppocr::logging::set_up_logging();

    let arg_matches = App::new("OCR service")
        .arg(
            Arg::with_name("config")
                .takes_value(true)
                .value_name("CONFIG")
                .help("Filename of configuration file")
                .required(true),
        )
        .arg(
            Arg::with_name("stream_id")
                .long("stream-id")
                .default_value("8840")
                .help("Instance ID number of the stream dumper service"),
        )
        .arg(
            Arg::with_name("stream_width")
                .long("stream-width")
                .default_value("1280")
                .help("Stream dumper's width of the output image"),
        )
        .arg(
            Arg::with_name("stream_height")
                .long("stream-height")
                .default_value("720")
                .help("Stream dumper's height of the output image"),
        )
        .arg(
            Arg::with_name("vnc_id")
                .long("vnc-id")
                .default_value("8855")
                .help("Instance ID number of the VNC server service"),
        )
        .arg(
            Arg::with_name("vnc_width")
                .long("vnc-width")
                .default_value("1024")
                .help("VNC server screen width"),
        )
        .arg(
            Arg::with_name("vnc_height")
                .long("vnc-height")
                .default_value("768")
                .help("VNC server screen height"),
        )
        .arg(
            Arg::with_name("tesseract_data_path")
                .long("tesseract-data-path")
                .default_value("/usr/share/tesseract-ocr/4.00/tessdata/")
                .help("Path of the Tesseract 'tessdata' directory."),
        )
        .arg(
            Arg::with_name("tesseract_language")
                .long("tesseract-language")
                .default_value("eng")
                .help("Tesseract language codes."),
        )
        .get_matches();

    let frame_reader = FrameReader::new(
        arg_matches.value_of("stream_id").unwrap().parse()?,
        arg_matches.value_of("stream_width").unwrap().parse()?,
        arg_matches.value_of("stream_height").unwrap().parse()?,
    )?;
    let vnc_client = VncClient::new(
        arg_matches.value_of("vnc_id").unwrap().parse()?,
        arg_matches.value_of("vnc_width").unwrap().parse()?,
        arg_matches.value_of("vnc_height").unwrap().parse()?,
    )?;
    let text_recognizer = TextRecognizer::new(
        arg_matches.value_of("tesseract_data_path").unwrap(),
        arg_matches.value_of("tesseract_language").unwrap(),
    )?;

    let config_text = std::fs::read_to_string(arg_matches.value_of("config").unwrap())?;
    let config: ProcessorConfig = toml::de::from_str(&config_text)?;

    let mut processor = Processor::new(frame_reader, vnc_client, text_recognizer, config);
    processor.run()?;

    Ok(())
}
