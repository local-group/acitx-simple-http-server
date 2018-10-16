
use clap;
use std::fs;
use std::net::IpAddr;
use std::path::PathBuf;
use std::env;
use std::error::Error;
use std::str::FromStr;

#[derive(Debug)]
pub struct Args {
    pub root: PathBuf,
    pub index: bool,
    pub upload: bool,
    pub sort: bool,
    pub cache: bool,
    pub range: bool,
    pub cert: Option<String>,
    pub certpass: Option<String>,
    pub cors: bool,
    pub ip: IpAddr,
    pub port: u16,
    pub threads: Option<usize>,
    pub auth: Option<String>,
    pub compression_exts: Vec<String>,
    pub try_file_404: Option<PathBuf>,
    pub silent: bool,
}

impl Args {
    pub fn from_clap() -> Args {
        let matches = clap::App::new("Simple HTTP(s) Server")
            .setting(clap::AppSettings::ColoredHelp)
            .version(crate_version!())
            .arg(clap::Arg::with_name("root")
                 .index(1)
                 .validator(|s| {
                     match fs::metadata(s) {
                         Ok(metadata) => {
                             if metadata.is_dir() { Ok(()) } else {
                                 Err("Not directory".to_owned())
                             }
                         },
                         Err(e) => Err(e.description().to_string())
                     }
                 })
                 .help("Root directory"))
            .arg(clap::Arg::with_name("index")
                 .short("i")
                 .long("index")
                 .help("Enable automatic render index page [index.html, index.htm]"))
            .arg(clap::Arg::with_name("upload")
                 .short("u")
                 .long("upload")
                 .help("Enable upload files (multiple select)"))
            .arg(clap::Arg::with_name("nosort")
                 .long("nosort")
                 .help("Disable directory entries sort (by: name, modified, size)"))
            .arg(clap::Arg::with_name("nocache")
                 .long("nocache")
                 .help("Disable http cache"))
            .arg(clap::Arg::with_name("norange")
                 .long("norange")
                 .help("Disable header::Range support (partial request)"))
            .arg(clap::Arg::with_name("cert")
                 .long("cert")
                 .takes_value(true)
                 .validator(|s| {
                     match fs::metadata(s) {
                         Ok(metadata) => {
                             if metadata.is_file() { Ok(()) } else {
                                 Err("Not a regular file".to_owned())
                             }
                         },
                         Err(e) => Err(e.description().to_string())
                     }
                 })
                 .help("TLS/SSL certificate (pkcs#12 format)"))
            .arg(clap::Arg::with_name("cors")
                 .long("cors")
                 .help("Enable CORS via the \"Access-Control-Allow-Origin\" header"))
            .arg(clap::Arg::with_name("certpass").
                 long("certpass")
                 .takes_value(true)
                 .help("TLS/SSL certificate password"))
            .arg(clap::Arg::with_name("ip")
                 .long("ip")
                 .takes_value(true)
                 .default_value("0.0.0.0")
                 .validator(|s| {
                     match IpAddr::from_str(&s) {
                         Ok(_) => Ok(()),
                         Err(e) => Err(e.description().to_string())
                     }
                 })
                 .help("IP address to bind"))
            .arg(clap::Arg::with_name("port")
                 .short("p")
                 .long("port")
                 .takes_value(true)
                 .default_value("8000")
                 .validator(|s| {
                     match s.parse::<u16>() {
                         Ok(_) => Ok(()),
                         Err(e) => Err(e.description().to_string())
                     }
                 })
                 .help("Port number"))
            .arg(clap::Arg::with_name("auth")
                 .short("a")
                 .long("auth")
                 .takes_value(true)
                 .validator(|s| {
                     let parts = s.splitn(2, ':').collect::<Vec<&str>>();
                     if parts.len() < 2 || parts.len() >= 2 && parts[1].len() < 1 {
                         Err("no password found".to_owned())
                     } else if parts[0].len() < 1 {
                         Err("no username found".to_owned())
                     } else {
                         Ok(())
                     }
                 })
                 .help("HTTP Basic Auth (username:password)"))
            .arg(clap::Arg::with_name("compress")
                 .short("c")
                 .long("compress")
                 .multiple(true)
                 .value_delimiter(",")
                 .takes_value(true)
                 .help(concat!(
                     "Enable file compression: gzip/deflate\n",
                     "    Example: -c=js,d.ts\n",
                     "    Note: disabled on partial request!"
                 )))
            .arg(clap::Arg::with_name("threads")
                 .short("t")
                 .long("threads")
                 .takes_value(true)
                 .validator(|s| {
                     match s.parse::<u8>() {
                         Ok(v) => {
                             if v > 0 { Ok(()) } else {
                                 Err("Not positive number".to_owned())
                             }
                         }
                         Err(e) => Err(e.description().to_string())
                     }
                 })
                 .help("How many worker threads"))
            .arg(clap::Arg::with_name("try-file-404")
                 .long("try-file")
                 .visible_alias("try-file-404")
                 .takes_value(true)
                 .value_name("PATH")
                 .validator(|s| {
                     match fs::metadata(s) {
                         Ok(metadata) => {
                             if metadata.is_file() { Ok(()) } else {
                                 Err("Not a file".to_owned())
                             }
                         },
                         Err(e) => Err(e.description().to_string())
                     }
                 })
                 .help("serve this file (server root relative) in place of missing files (useful for single page apps)"))
            .arg(clap::Arg::with_name("silent")
                 .long("silent")
                 .short("s")
                 .takes_value(false)
                 .help("Disable all outputs"))
            .get_matches();

        let root = matches
            .value_of("root")
            .map(PathBuf::from)
            .unwrap_or_else(|| env::current_dir().unwrap());
        let index = matches.is_present("index");
        let upload = matches.is_present("upload");
        let sort = !matches.is_present("nosort");
        let cache = !matches.is_present("nocache");
        let range = !matches.is_present("norange");
        let cert = matches
            .value_of("cert")
            .map(|s| s.to_string());
        let certpass = matches
            .value_of("certpass")
            .map(|s| s.to_string());
        let cors = matches.is_present("cors");
        let ip = matches
            .value_of("ip")
            .map(|s| IpAddr::from_str(s).unwrap())
            .unwrap();
        let port = matches
            .value_of("port")
            .unwrap()
            .parse::<u16>()
            .unwrap();
        let auth = matches
            .value_of("auth")
            .map(|s| s.to_string());
        let compress = matches.values_of_lossy("compress");
        let threads = matches
            .value_of("threads")
            .map(|s| s.parse::<usize>().unwrap());
        let try_file_404 = matches
            .value_of("try-file-404")
            .map(PathBuf::from);

        let compression_exts = compress.clone()
            .unwrap_or_default()
            .iter()
            .map(|s| format!("*.{}", s))
            .collect::<Vec<String>>();
        let silent = matches.is_present("silent");

        Args {
            root,
            index,
            upload,
            sort,
            cache,
            range,
            cert,
            certpass,
            cors,
            ip,
            port,
            threads,
            auth,
            compression_exts,
            try_file_404,
            silent,
        }
    }
}
