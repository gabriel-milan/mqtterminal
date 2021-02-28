# MQTTerminal Server

Run shell commands asynchronously over MQTT with very low setup

[![Build Status](https://travis-ci.com/gabriel-milan/mqtterminal-server.svg?branch=master)](https://travis-ci.com/gabriel-milan/mqtterminal-server)
[![Crate](https://img.shields.io/crates/v/mqtterminal-server)](https://crates.io/crates/mqtterminal-server)

With [mqtterminal-client](https://github.com/gabriel-milan/mqtterminal-client), you'll be able to execute commands on a remote machine running the [mqtterminal-server](https://github.com/gabriel-milan/mqtterminal-server) and get the output from it. For that to happen, you must configure both ends to connect to the same MQTT broker instance and subscribed to the very same topic (see "Usage" below).

**Disclaimer**: When using a public or non-authenticated MQTT broker, anyone on the internet is able to sniff into your topic and both execute commands and get outputs from your commands, so be careful when using it. It's **not** safe when you have no control over the environment.

## Installation

### Option #1 (recommended) - Download the binaries

* Download the binaries for your architecture [here](https://github.com/gabriel-milan/mqtterminal-server/releases)
* Uncompress it
* There it is!

### Option #2 - Build it for yourself

* You must have Rust installed, if you don't, the best way to do this is through [rustup](https://rustup.rs/)
* Clone this repository
* Then run `cargo install --path .`

## Usage
```
$ mqtterminal-server --help
mqtterminal-server 0.1.0
Server side for the MQTTerminal project at https://github.com/gabriel-milan/mqtterminal

USAGE:
    mqtterminal-server [FLAGS] [OPTIONS] --topic <TOPIC>

FLAGS:
    -h, --help       Prints help information
    -v               Sets the level of verbosity
    -V, --version    Prints version information

OPTIONS:
    -b, --broker_url <BROKER_URL>      Hostname of the broker where you'll get your messages (default:
                                       tcp://broker.hivemq.com:1883)
    -n, --client_name <CLIENT_NAME>    Client name when connecting to the broker, must be unique (default: randomly
                                       generated UUIDv4)
    -t, --topic <TOPIC>                Topic for publishing/subscription. On public brokers, anyone using this topic is
                                       able to intercept MQTTerminal.
```
