#[macro_use]
extern crate log;

pub mod conf {
    use failure::{format_err, ResultExt};
    use glob::glob;
    use serde_derive::{Deserialize, Serialize};
    use std::path::PathBuf;
    use structopt::StructOpt;
    use url::Url;

    #[derive(Debug, StructOpt)]
    #[structopt(rename_all = "kebab-case")]
    pub struct Args {
        #[structopt(
            short,
            long,
            help = "listen address, such as 0.0.0.0, 127.0.0.1, 192.168.1.2"
        )]
        pub host: Option<String>,
        #[structopt(short, long, help = "listen port")]
        pub port: Option<u16>,
        #[structopt(short, long, help = "virtual host. eg. localhost, example.com")]
        pub domain: Option<String>,
        #[structopt(
            short = "r",
            long,
            parse(try_from_str = "parse_reverse_proxy_mapping"),
            help = "eg. /path/to:http://localhost:3000/path/to"
        )]
        pub reverse_proxy: Vec<ReverseProxyMapping>,
        #[structopt(
            long,
            parse(from_os_str),
            help = "a nginx conf file path to which this will write out"
        )]
        pub nginx_conf: Option<PathBuf>,
        #[structopt(
            long,
            default_value = "/conf",
            parse(from_str = "parse_path_without_trailing_slash")
        )]
        pub config_dir: PathBuf,
        #[structopt(flatten)]
        pub verbose: clap_verbosity_flag::Verbosity,
    }

    impl Args {
        pub fn from_args() -> Args {
            <Args as StructOpt>::from_args()
        }
    }

    pub fn parse_reverse_proxy_mapping(s: &str) -> Result<ReverseProxyMapping, failure::Error> {
        ReverseProxyMapping::parse(s)
    }

    pub fn parse_path_without_trailing_slash(s: &str) -> PathBuf {
        PathBuf::from(s.trim_end_matches("/"))
    }

    #[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
    pub struct ReverseProxyMapping {
        pub path: String,
        #[serde(with = "url_serde")]
        pub url: Url,
    }

    impl ReverseProxyMapping {
        pub fn parse(s: &str) -> Result<ReverseProxyMapping, failure::Error> {
            let i = s
                .find(":")
                .ok_or_else(|| format_err!("missing separator ':' in {}", s))?;
            let (path, url) = s.split_at(i);
            let url = url.trim_start_matches(":");
            Ok(ReverseProxyMapping {
                path: path.into(),
                url: Url::parse(url)
                    .with_context(|_| format!("Failed to parse as URL: {}", url.to_owned()))?,
            })
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct RawAppConfig {
        host: Option<String>,
        port: Option<u16>,
        domain: Option<String>,
        #[serde(default)]
        reverse_proxy: Vec<ReverseProxyMapping>,
        nginx_conf: Option<PathBuf>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AppConfig {
        pub host: String,
        pub port: u16,
        pub domain: Option<String>,
        #[serde(default)]
        pub reverse_proxy: Vec<ReverseProxyMapping>,
        pub nginx_conf: PathBuf,
    }

    impl AppConfig {
        /// panic: error in config files or CLI arguments
        pub fn from_args_and_config(args: Args) -> Result<AppConfig, failure::Error> {
            let mut settings = config::Config::default();
            let config_dir = format!("{}/*", args.config_dir.display());
            debug!("config_dir: {}", config_dir);
            settings.merge(
                glob(&config_dir)?
                    .map(|path| {
                        path.map(|path| {
                            info!("load config file: {}", path.display());
                            config::File::from(path)
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            )?;
            trace!("settings: {:#?}", settings);

            let RawAppConfig {
                host: rac_host,
                port: rac_port,
                domain: rac_domain,
                reverse_proxy: rac_reverse_proxy,
                nginx_conf: rac_nginx_conf,
            } = {
                let raw_app_config = settings.try_into()?;
                debug!("raw_app_config: {:#?}", raw_app_config);
                raw_app_config
            };

            let Args {
                host: args_host,
                port: args_port,
                domain: args_domain,
                reverse_proxy: args_reverse_proxy,
                nginx_conf: args_nginx_conf,
                config_dir: _,
                verbose: _,
            } = args;

            Ok(AppConfig {
                host: args_host.or(rac_host).unwrap_or_else(|| "0.0.0.0".into()),
                port: args_port.or(rac_port).unwrap_or(10080),
                domain: args_domain.or(rac_domain),
                reverse_proxy: args_reverse_proxy
                    .into_iter()
                    .chain(rac_reverse_proxy.into_iter())
                    .collect(),
                nginx_conf: args_nginx_conf
                    .or(rac_nginx_conf)
                    .unwrap_or_else(|| PathBuf::from("/etc/nginx/conf.d/default.conf")),
            })
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;

        #[test]
        fn config_dir_default_not_specified() {
            use structopt::StructOpt;
            let cli_args: &[&str] = &["test"];
            let args = Args::from_iter_safe(cli_args.iter()).unwrap();
            assert_eq!(
                "/conf".to_string(),
                format!("{}", args.config_dir.display())
            );
        }

        #[test]
        fn config_dir_default_omit_value() {
            use structopt::StructOpt;
            let cli_args: &[&str] = &["test", "--config-dir"];
            let args = Args::from_iter_safe(cli_args.iter()).unwrap();
            assert_eq!(
                "/conf".to_string(),
                format!("{}", args.config_dir.display())
            );
        }

        #[test]
        fn config_dir_custom_relative_path() {
            use structopt::StructOpt;
            let cli_args: &[&str] = &["test", "--config-dir", "./conf"];
            let args = Args::from_iter_safe(cli_args.iter()).unwrap();
            assert_eq!(
                "./conf".to_string(),
                format!("{}", args.config_dir.display())
            );
        }

        #[test]
        fn config_dir_custom_absolute_path() {
            use structopt::StructOpt;
            let cli_args: &[&str] = &["test", "--config-dir", "/path/to/conf"];
            let args = Args::from_iter_safe(cli_args.iter()).unwrap();
            assert_eq!(
                "/path/to/conf".to_string(),
                format!("{}", args.config_dir.display())
            );
        }

        #[test]
        fn config_dir_custom_relative_path_with_trailing_slash() {
            use structopt::StructOpt;
            let cli_args: &[&str] = &["test", "--config-dir", "./conf/"];
            let args = Args::from_iter_safe(cli_args.iter()).unwrap();
            assert_eq!(
                "./conf".to_string(),
                format!("{}", args.config_dir.display())
            );
        }

        #[test]
        fn config_dir_custom_absolute_path_with_trailing_slash() {
            use structopt::StructOpt;
            let cli_args: &[&str] = &["test", "--config-dir", "/path/to/conf/"];
            let args = Args::from_iter_safe(cli_args.iter()).unwrap();
            assert_eq!(
                "/path/to/conf".to_string(),
                format!("{}", args.config_dir.display())
            );
        }

        #[test]
        fn nginx_conf_path_default_args() {
            use structopt::StructOpt;
            let cli_args: &[&str] = &["test"];
            let args = Args::from_iter_safe(cli_args.iter()).unwrap();
            assert_eq!(None, args.nginx_conf);
        }

        #[test]
        fn nginx_conf_path_custom_args() {
            use structopt::StructOpt;
            let cli_args: &[&str] = &["test", "--nginx-conf", "/etc/nginx/conf.d/custom.conf"];
            let args = Args::from_iter_safe(cli_args.iter()).unwrap();
            assert_eq!(
                Some(PathBuf::from("/etc/nginx/conf.d/custom.conf")),
                args.nginx_conf
            );
        }

        #[test]
        #[should_panic(expected = "--nginx-conf without value")]
        fn nginx_conf_path_empty_args() {
            use structopt::StructOpt;
            let cli_args: &[&str] = &["test", "--nginx-conf"];
            Args::from_iter_safe(cli_args.iter()).expect("--nginx-conf without value");
        }

        #[test]
        fn nginx_conf_path_default_app_config() {
            use structopt::StructOpt;
            let cli_args: &[&str] = &["test"];
            let args = Args::from_iter_safe(cli_args.iter()).unwrap();
            let app_config = AppConfig::from_args_and_config(args).unwrap();
            assert_eq!(
                PathBuf::from("/etc/nginx/conf.d/default.conf"),
                app_config.nginx_conf
            );
        }

        #[test]
        fn nginx_conf_path_custom_app_config() {
            use structopt::StructOpt;
            let cli_args: &[&str] = &["test", "--config-dir", "./tests/conf_nginx_dir"];
            let args = Args::from_iter_safe(cli_args.iter()).unwrap();
            let app_config = AppConfig::from_args_and_config(args).unwrap();
            assert_eq!(
                PathBuf::from("./tmp/nginx_default.conf"),
                app_config.nginx_conf
            );
        }
    }
}

use failure::ResultExt;
use std::fs;
use std::io::{self, Write};

fn main() -> Result<(), exitfailure::ExitFailure> {
    let args = conf::Args::from_args();
    env_logger::builder()
        .filter_level(args.verbose.log_level().to_level_filter())
        .init();
    debug!("args: {:#?}", args);
    let app_config = conf::AppConfig::from_args_and_config(args).context("Load config")?;
    debug!("app_config: {:#?}", app_config);

    let mut writer = io::BufWriter::new(
        fs::File::create(app_config.nginx_conf.as_path())
            .with_context(|err| format!("{}: {}", err, app_config.nginx_conf.display()))?,
    );
    write!(
        writer,
        "{}",
        render_nginx_conf(
            &app_config.host,
            app_config.port,
            app_config.domain.as_ref().map(|s| s.as_str()),
            &app_config.reverse_proxy
        )
    )?;

    Ok(())
}

pub fn render_nginx_conf(
    host: &str,
    port: u16,
    domain: Option<&str>,
    reverse_proxy_mappings: &[conf::ReverseProxyMapping],
) -> String {
    let reverse_proxy_locations =
        reverse_proxy_mappings
            .iter()
            .fold(String::new(), |mut buf, rp| {
                buf.push_str(&format!(
                    r#"
    location {} {{
        proxy_pass {};
    }}
"#,
                    rp.path, rp.url
                ));
                buf
            });

    let conf = format!(
        r#"
server {{
    listen {}:{};
    server_name {};

    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-Host $http_host;
    proxy_set_header X-Forwarded-Server $host;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;

    {}
}}
"#,
        host,
        port,
        domain.unwrap_or("localhost"),
        reverse_proxy_locations,
    );

    conf
}
