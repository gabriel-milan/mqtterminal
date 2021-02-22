mod logger;

extern crate clap;
extern crate paho_mqtt as mqtt;

use clap::{App, Arg};
use std::{process, str, thread, time::Duration};
use uuid::Uuid;

const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
const QOS: i32 = 2;
const EXIT_CODE_FAILURE: i32 = 1;
const COMMAND_PREFIX: &str = "COMMAND/";

fn try_reconnect(client: &mqtt::Client, logger: &logger::Logger) -> bool {
    logger.warning("Connection lost. Will retry reconnection...");
    for i in 0..12 {
        thread::sleep(Duration::from_millis(5000));
        logger.info(format!("Retrying connection (attempt #{})", i + 1));
        if client.reconnect().is_ok() {
            logger.info("Successfully reconnected!");
            return true;
        }
    }
    logger.fatal("Unable to reconnect after several attempts.");
    false
}

fn subscribe(client: &mqtt::Client, m_logger: &logger::Logger, topic: &str) {
    if let Err(e) = client.subscribe(topic, QOS) {
        m_logger.fatal(format!("Failed to subscribe to topic {}: {:?}", topic, e));
        process::exit(1);
    }
}

fn main() {
    let matches = App::new("mqtterminal-server")
        .version(VERSION.unwrap_or("unknown"))
        .about("Server side for the MQTTerminal project at https://github.com/gabriel-milan/mqtterminal")
        .arg(
            Arg::with_name("broker_url")
                .short("b")
                .long("broker_url")
                .value_name("BROKER_URL")
                .help("Hostname of the broker where you'll get your messages (default: tcp://broker.hivemq.com:1883)")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("client_name")
                .short("n")
                .long("client_name")
                .value_name("CLIENT_NAME")
                .help("Client name when connecting to the broker, must be unique (default: randomly generated UUIDv4)")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("topic")
                .short("t")
                .long("topic")
                .value_name("TOPIC")
                .help("Topic for publishing/subscription. On public brokers, anyone using this topic is able to intercept MQTTerminal.")
                .required(true)
                .takes_value(true)
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .get_matches();

    let broker_url = matches
        .value_of("broker_url")
        .unwrap_or("tcp://broker.hivemq.com:1883");

    let client_name = match matches.value_of("client_name") {
        Some(x) => x.to_string(),
        _ => Uuid::new_v4().to_string(),
    };

    let topic = matches.value_of("topic").unwrap();

    let verbosity = matches.occurrences_of("v");

    let m_logger = logger::Logger::new(verbosity as i8);

    m_logger.verbose("--------------------------------");
    m_logger.verbose(format!(
        "* MQTTerminal Server version {}",
        VERSION.unwrap_or("unknown")
    ));
    m_logger.verbose(format!("* Broker URL: {}", broker_url));
    m_logger.verbose(format!("* Client ID: {}", client_name));
    m_logger.verbose(format!("* Topic: {}", topic));
    m_logger.verbose(format!("* QOS: {}", QOS));
    m_logger.warning("* Be careful when using public/unauthenticated brokers for MQTTerminal!!!");
    m_logger.verbose("--------------------------------");

    m_logger.debug("Setting up MQTT client creation options...");
    let create_opts = mqtt::CreateOptionsBuilder::new()
        .server_uri(broker_url)
        .client_id(client_name)
        .finalize();
    m_logger.verbose("Successfully setup MQTT client creation options");

    m_logger.debug("Setting up MQTT client...");
    let mut client = mqtt::Client::new(create_opts).unwrap_or_else(|err| {
        m_logger.fatal(format!("Error while creating MQTT client: {:?}", err));
        process::exit(EXIT_CODE_FAILURE);
    });
    m_logger.verbose("Successfully setup MQTT client");

    m_logger.debug("Setting up MQTT consumer...");
    let rx = client.start_consuming();
    m_logger.verbose("Successfully setup MQTT consumer");

    m_logger.debug("Setting up MQTT connection options...");
    let lwt = mqtt::MessageBuilder::new()
        .topic(topic)
        .payload("Server has lost connection")
        .finalize();
    let conn_opts = mqtt::ConnectOptionsBuilder::new()
        .keep_alive_interval(Duration::from_secs(20))
        .clean_session(false)
        .will_message(lwt)
        .finalize();
    m_logger.verbose("Successfully setup MQTT connection options");

    m_logger.info("Connecting to the MQTT broker...");
    if let Err(e) = client.connect(conn_opts) {
        m_logger.fatal(format!("Unable to connect to broker: {:?}", e));
        process::exit(EXIT_CODE_FAILURE);
    }
    m_logger.debug("Successfully connected to the MQTT broker.");

    m_logger.debug(format!("Subscribing to topic {}...", topic));
    subscribe(&client, &m_logger, topic);
    m_logger.verbose(format!("Successfully subscribed to topic {}", topic));

    m_logger.info("Started processing requests...");
    for msg in rx.iter() {
        if let Some(msg) = msg {
            let cmd: &str;
            let payload = msg.payload_str();
            if payload.starts_with(COMMAND_PREFIX) {
                match payload.strip_prefix(COMMAND_PREFIX) {
                    Some(v) => cmd = v,
                    _ => {
                        m_logger.error("Got an unexpected error while parsing message payload.");
                        process::exit(EXIT_CODE_FAILURE);
                    }
                }
                let output = if cfg!(target_os = "windows") {
                    match process::Command::new("cmd").args(&["/C", cmd]).output() {
                        Ok(v) => v,
                        Err(e) => {
                            m_logger.error(format!("Failed to execute command {}: {}", cmd, e));
                            process::exit(EXIT_CODE_FAILURE);
                        }
                    }
                } else {
                    match process::Command::new("sh").arg("-c").arg(cmd).output() {
                        Ok(v) => v,
                        Err(e) => {
                            m_logger.error(format!("Failed to execute command {}: {}", cmd, e));
                            process::exit(EXIT_CODE_FAILURE);
                        }
                    }
                };
                let out_stdout = match str::from_utf8(&output.stdout) {
                    Ok(v) => v,
                    Err(e) => {
                        m_logger
                            .error(format!("Failed to parse stdout for command {}: {}", cmd, e));
                        process::exit(EXIT_CODE_FAILURE);
                    }
                };
                let out_stderr = match str::from_utf8(&output.stderr) {
                    Ok(v) => v,
                    Err(e) => {
                        m_logger
                            .error(format!("Failed to parse stderr for command {}: {}", cmd, e));
                        process::exit(EXIT_CODE_FAILURE);
                    }
                };
                m_logger.debug("Sending output to client...");
                let msg: mqtt::Message;
                if output.status.success() {
                    msg = mqtt::Message::new(topic, format!("OUTPUT/{}", out_stdout), QOS);
                    m_logger.debug("Command successfully executed, sending stdout...");
                } else {
                    msg = mqtt::Message::new(topic, format!("OUTPUT/{}", out_stderr), QOS);
                    m_logger.debug("Command has failed, sending stderr...");
                }
                match client.publish(msg) {
                    Ok(_) => m_logger.debug("Successfully sent output!"),
                    Err(e) => m_logger.error(format!("Failed to send output: {:?}", e)),
                }
            }
        } else if !client.is_connected() {
            if try_reconnect(&client, &m_logger) {
                m_logger.info("Resubscribing...");
                subscribe(&client, &m_logger, topic);
            } else {
                break;
            }
        }
    }

    if client.is_connected() {
        m_logger.info("Disconnecting...");
        match client.unsubscribe(topic) {
            Ok(_) => m_logger.debug("Successfully unsubscribed!"),
            Err(e) => {
                m_logger.error(format!("Failed to unsubscribe: {}", e));
                process::exit(EXIT_CODE_FAILURE);
            }
        };
        match client.disconnect(None) {
            Ok(_) => m_logger.debug("Successfully disconnected from broker!"),
            Err(e) => {
                m_logger.error(format!("Failed to disconnect from broker: {}", e));
                process::exit(EXIT_CODE_FAILURE);
            }
        };
    }
}
