mod logger;

extern crate clap;
extern crate paho_mqtt as mqtt;

use clap::{App, Arg};
use std::{
    io::{stdin, stdout, Write},
    process,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use uuid::Uuid;

const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
const QOS: i32 = 2;
const EXIT_CODE_FAILURE: i32 = 1;
const OUTPUT_PREFIX: &str = "OUTPUT/";

fn subscribe(client: &Arc<Mutex<mqtt::Client>>, m_logger: &logger::Logger, topic: &str) {
    if let Err(e) = match client.lock() {
        Ok(v) => v,
        Err(e) => {
            m_logger.fatal(format!(
                "Failed to lock client mutex for subscription: {}",
                e
            ));
            process::exit(EXIT_CODE_FAILURE);
        }
    }
    .subscribe(topic, QOS)
    {
        m_logger.fatal(format!("Failed to subscribe to topic {}: {:?}", topic, e));
        process::exit(1);
    }
}

fn try_reconnect(client: &Arc<Mutex<mqtt::Client>>, logger: &logger::Logger) -> bool {
    logger.warning("Connection lost. Will retry reconnection...");
    for i in 0..12 {
        thread::sleep(Duration::from_millis(5000));
        logger.info(format!("Retrying connection (attempt #{})", i + 1));
        if match client.lock() {
            Ok(v) => v,
            Err(e) => {
                logger.fatal(format!(
                    "Failed to lock client mutex for reconnection: {}",
                    e
                ));
                process::exit(EXIT_CODE_FAILURE);
            }
        }
        .reconnect()
        .is_ok()
        {
            logger.info("Successfully reconnected!");
            return true;
        }
    }
    logger.fatal("Unable to reconnect after several attempts.");
    false
}

fn main() {
    let matches = App::new("mqtterminal-client")
        .version(VERSION.unwrap_or("unknown"))
        .about("Client side for the MQTTerminal project at https://github.com/gabriel-milan/mqtterminal")
        .arg(
            Arg::with_name("broker_url")
                .short("b")
                .long("broker_url")
                .value_name("BROKER_URL")
                .help("Hostname of the broker where you'll send your messages (default: tcp://broker.hivemq.com:1883)")
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

    let topic = matches.value_of("topic").unwrap().to_string();

    let verbosity = matches.occurrences_of("v");

    let m_logger = logger::Logger::new(verbosity as i8);

    m_logger.verbose("--------------------------------");
    m_logger.verbose(format!(
        "* MQTTerminal Client version {}",
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
    let client = Arc::new(Mutex::new(mqtt::Client::new(create_opts).unwrap_or_else(
        |err| {
            m_logger.fatal(format!("Error while creating MQTT client: {:?}", err));
            process::exit(EXIT_CODE_FAILURE);
        },
    )));
    m_logger.verbose("Successfully setup MQTT client");

    m_logger.debug("Setting up MQTT consumer...");
    let rx = match client.lock() {
        Ok(v) => v,
        Err(e) => {
            m_logger.fatal(format!("Failed to lock client mutex for RX: {}", e));
            process::exit(EXIT_CODE_FAILURE);
        }
    }
    .start_consuming();
    m_logger.verbose("Successfully setup MQTT consumer");

    m_logger.debug("Setting up MQTT connection options...");
    let conn_opts = mqtt::ConnectOptionsBuilder::new()
        .keep_alive_interval(Duration::from_secs(20))
        .clean_session(true)
        .finalize();
    m_logger.verbose("Successfully setup MQTT connection options");

    m_logger.info("Connecting to the MQTT broker...");
    if let Err(e) = match client.lock() {
        Ok(v) => v,
        Err(e) => {
            m_logger.fatal(format!("Failed to lock client mutex for connection: {}", e));
            process::exit(EXIT_CODE_FAILURE);
        }
    }
    .connect(conn_opts)
    {
        m_logger.fatal(format!("Unable to connect to broker: {:?}", e));
        process::exit(EXIT_CODE_FAILURE);
    }
    m_logger.debug("Successfully connected to the MQTT broker.");

    m_logger.debug(format!("Subscribing to topic {}...", topic));
    subscribe(&client, &m_logger, &topic);
    m_logger.verbose(format!("Successfully subscribed to topic {}", topic));

    let thread_client = client.clone();
    let thread_topic = topic.clone();
    thread::spawn(move || {
        for msg in rx.iter() {
            if let Some(msg) = msg {
                let output: &str;
                let payload = msg.payload_str();
                if payload.starts_with(OUTPUT_PREFIX) {
                    match payload.strip_prefix(OUTPUT_PREFIX) {
                        Some(v) => output = v,
                        _ => {
                            m_logger
                                .error("Got an unexpected error while parsing message payload.");
                            process::exit(EXIT_CODE_FAILURE);
                        }
                    }
                    print!("\n{}", output);
                }
            } else if !match thread_client.lock() {
                Ok(v) => v,
                Err(e) => {
                    m_logger.fatal(format!(
                        "Failed to lock client mutex for checking connection: {}",
                        e
                    ));
                    process::exit(EXIT_CODE_FAILURE);
                }
            }
            .is_connected()
            {
                if try_reconnect(&thread_client, &m_logger) {
                    m_logger.info("Resubscribing...");
                    subscribe(&thread_client, &m_logger, &thread_topic);
                } else {
                    break;
                }
            }
        }
    });

    m_logger.info(
        "Everything set! You can quit at anytime by typing Ctrl+C or sending command \"exit\"",
    );

    let mut cmd: String;
    loop {
        cmd = String::new();
        print!("$ ");
        let _ = stdout().flush();
        match stdin().read_line(&mut cmd) {
            Ok(t) => t,
            Err(e) => {
                m_logger.error(format!("Failed to parse input: {:?}", e));
                process::exit(EXIT_CODE_FAILURE);
            }
        };
        if let Some('\n') = cmd.chars().next_back() {
            cmd.pop();
        }
        if let Some('\r') = cmd.chars().next_back() {
            cmd.pop();
        }
        let msg = mqtt::Message::new(&topic, format!("COMMAND/{}", cmd.clone()), QOS);
        if cmd.eq("exit") {
            break;
        } else if !cmd.eq("") {
            m_logger.debug(format!("Sending command {}...", cmd));
            match match client.lock() {
                Ok(v) => v,
                Err(e) => {
                    m_logger.fatal(format!("Failed to lock client mutex for publishing: {}", e));
                    process::exit(EXIT_CODE_FAILURE);
                }
            }
            .publish(msg)
            {
                Ok(_) => m_logger.debug(format!("Successfully sent command {}", cmd)),
                Err(e) => m_logger.error(format!("Failed to send command {}: {:?}", cmd, e)),
            }
        }
    }

    m_logger.debug("Disconnecting from broker...");
    let m_client = match client.lock() {
        Ok(v) => v,
        Err(e) => {
            m_logger.fatal(format!(
                "Failed to lock client mutex for disconnecting: {}",
                e
            ));
            process::exit(EXIT_CODE_FAILURE);
        }
    };
    match m_client.disconnect(None) {
        Ok(_) => m_logger.debug("Successfully disconnected!"),
        Err(e) => {
            m_logger.error(format!("Failed to disconnect from broker: {}", e));
            process::exit(EXIT_CODE_FAILURE);
        }
    };
}
