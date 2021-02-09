use clap::{App, Arg};
use slog_scope::info;

fn main() -> anyhow::Result<()> {
    tppocr::logging::set_up_logging();

    let arg_matches = App::new("stream frame dumper")
        .arg(
            Arg::with_name("input")
                .value_name("INPUT")
                .takes_value(true)
                .required(true)
                .help("URL of stream to be passed to ffmpeg's libav suite"),
        )
        .arg(Arg::with_name("get_url").long("get-url").help(
            "Interpret INPUT as a webpage link and get the actual stream URL using youtube-dl",
        ))
        .arg(
            Arg::with_name("format")
                .long("format")
                .value_name("FORMAT")
                .takes_value(true)
                .default_value("720p60")
                .help("When --get-url is specified, resolution format of the stream"),
        )
        .arg(
            Arg::with_name("width")
                .long("width")
                .default_value("1280")
                .help("Width of output image"),
        )
        .arg(
            Arg::with_name("height")
                .long("height")
                .default_value("720")
                .help("Height of output image"),
        )
        .arg(
            Arg::with_name("id")
                .long("id")
                .default_value("8840")
                .help("Instance ID number for shared memory and port number"),
        )
        .arg(Arg::with_name("skip_sleep").long("skip-sleep").help(
            "Don't sleep to account for presentation time; \
            read the input as fast as possible.",
        ))
        .arg(
            Arg::with_name("loop")
                .long("loop")
                .help("Loop the input source (for debugging)"),
        )
        .get_matches();

    let mut url = arg_matches.value_of("input").unwrap().to_owned();

    if arg_matches.is_present("get_url") {
        url = tppocr::stream_url::get_stream_url(&url, arg_matches.value_of("format").unwrap())?;
        info!("got stream url"; "url" => &url);
    }
    ffmpeg_next::init()?;

    let mut server = tppocr::frame::FrameDumper::new(
        url,
        arg_matches.value_of("id").unwrap().parse()?,
        arg_matches.value_of("width").unwrap().parse()?,
        arg_matches.value_of("height").unwrap().parse()?,
    )?;

    if arg_matches.is_present("loop") {
        server.set_infinite_loop(true);
    }

    if arg_matches.is_present("skip_sleep") {
        server.set_skip_sleep(true);
    }

    server.run()
}
