use clap::{App, Arg};

fn main() -> anyhow::Result<()> {
    tppocr::logging::set_up_logging();

    let arg_matches = App::new("VNC debug image server")
        .arg(
            Arg::with_name("width")
                .long("width")
                .default_value("1024")
                .help("Width of screen"),
        )
        .arg(
            Arg::with_name("height")
                .long("height")
                .default_value("768")
                .help("Height of screen"),
        )
        .arg(
            Arg::with_name("id")
                .long("id")
                .default_value("8855")
                .help("Instance ID number for shared memory and port number"),
        )
        .get_matches();

    let mut server = tppocr::vnc::VncServer::new(
        arg_matches.value_of("id").unwrap().parse()?,
        arg_matches.value_of("width").unwrap().parse()?,
        arg_matches.value_of("height").unwrap().parse()?,
    )?;
    server.run()
}
