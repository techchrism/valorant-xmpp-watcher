use std::{io::{Read, Write}, net::{TcpStream}, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};

use native_tls::{TlsConnector, TlsStream};
use serde::Serialize;

const KEEPALIVE_DURATION: Duration = Duration::from_secs(150);

#[derive(Serialize)]
struct DataLogLine {
    #[serde(rename = "type")]
    event_type: String,
    time: u128,
    data: String
}

fn write(
    contents: &str, 
    stream: &mut TlsStream<TcpStream>,
    mut file_writer: impl Write
) -> anyhow::Result<()> {
    stream.write_all(contents.as_bytes())?;

    file_writer.write_all(serde_json::to_string(&DataLogLine {
        event_type: "outgoing".to_string(),
        time: SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
        data: contents.to_string()
    })?.as_bytes())?;
    file_writer.write_all(b"\n")?;

    Ok(())
}

fn read_with_keepalive(
    stream: &mut TlsStream<TcpStream>,
    last_keepalive_sent: &mut Instant,
    mut file_writer: impl Write
)-> anyhow::Result<String> {
    let mut buffer = [0u8; 16 * 1024];
    loop {
        let duration_until_next_keepalive = KEEPALIVE_DURATION.saturating_sub(last_keepalive_sent.elapsed());

        if duration_until_next_keepalive.is_zero() {
            write(".", stream, &mut file_writer)?;
            stream.flush()?;
            *last_keepalive_sent = Instant::now();
            continue;
        }

        stream.get_ref().set_read_timeout(Some(duration_until_next_keepalive))?;

        match stream.read(&mut buffer) {
            Ok(0) => anyhow::bail!("Connection closed"),
            Ok(n) => {
                let read_string = String::from_utf8(buffer[..n].to_vec())?;
                file_writer.write_all(serde_json::to_string(&DataLogLine {
                    event_type: "incoming".to_string(),
                    time: SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
                    data: read_string.clone()
                })?.as_bytes())?;
                file_writer.write_all(b"\n")?;
                return Ok(read_string);
            },
            Err(e) if matches!(e.kind(), std::io::ErrorKind::TimedOut) || matches!(e.kind(), std::io::ErrorKind::WouldBlock) => continue,
            Err(e) => return Err(e.into())
        }
    }
}

pub fn xmpp_connect(
    xmpp_host: &str,
    xmpp_port: u16,
    mut file_writer: impl Write,
    affinity_domain: &str,
    access_token: &str,
    pas_token: &str,
    entitlements_token: &str
) -> anyhow::Result<()> {
    let tcp_stream = TcpStream::connect((xmpp_host, xmpp_port))?;
    let connector = TlsConnector::new()?;
    let mut stream = connector.connect(xmpp_host, tcp_stream)?;

    let mut last_keepalive_sent = Instant::now();

    println!("Connected!");

    write(&format!(
        r#"<?xml version="1.0"?><stream:stream to="{}.pvp.net" version="1.0" xmlns:stream="http://etherx.jabber.org/streams">"#,
        affinity_domain
    ), &mut stream, &mut file_writer)?;
    read_with_keepalive(&mut stream, &mut last_keepalive_sent, &mut file_writer)?;

    write(&format!(
        r#"<auth mechanism="X-Riot-RSO-PAS" xmlns="urn:ietf:params:xml:ns:xmpp-sasl"><rso_token>{}</rso_token><pas_token>{}</pas_token></auth>"#,
        access_token,
        pas_token
    ), &mut stream, &mut file_writer)?;
    read_with_keepalive(&mut stream, &mut last_keepalive_sent, &mut file_writer)?;

    write(&format!(
        r#"<?xml version="1.0"?><stream:stream to="{}.pvp.net" version="1.0" xmlns:stream="http://etherx.jabber.org/streams">"#,
        affinity_domain
    ), &mut stream, &mut file_writer)?;
    loop {
        let response = read_with_keepalive(&mut stream, &mut last_keepalive_sent, &mut file_writer)?;
        if response.contains("stream:features") {
            break;
        }
    }

    write(r#"<iq id="_xmpp_bind1" type="set"><bind xmlns="urn:ietf:params:xml:ns:xmpp-bind"></bind></iq>"#, &mut stream, &mut file_writer)?;
    read_with_keepalive(&mut stream, &mut last_keepalive_sent, &mut file_writer)?;

    write(r#"<iq id="_xmpp_session1" type="set"><session xmlns="urn:ietf:params:xml:ns:xmpp-session"/></iq>"#, &mut stream, &mut file_writer)?;
    read_with_keepalive(&mut stream, &mut last_keepalive_sent, &mut file_writer)?;

    write(&format!(
        r#"<iq id="xmpp_entitlements_0" type="set"><entitlements xmlns="urn:riotgames:entitlements"><token xmlns="">{}</token></entitlements></iq>"#,
        entitlements_token
    ), &mut stream, &mut file_writer)?;
    read_with_keepalive(&mut stream, &mut last_keepalive_sent, &mut file_writer)?;

    println!("Finished authentication! Requesting roster and chats...");

    write(r#"<iq type="get" id="1"><query xmlns="jabber:iq:riotgames:roster" last_state="true"/></iq><iq type="get" id="recent_convos_3"><query xmlns="jabber:iq:riotgames:archive:list"/></iq>"#, &mut stream, &mut file_writer)?;

    println!("Requesting presence...");

    write(r#"<presence/>"#, &mut stream, &mut file_writer)?;

    println!("Done! Continually reading...");

    loop {
        read_with_keepalive(&mut stream, &mut last_keepalive_sent, &mut file_writer)?;
    }
}
