//! LAN play between browsers (M2k): a browser can't send UDP, so with the relay Off the browser build connects by
//! WebRTC instead of iroh, with no server at all (no STUN, no signalling: host candidates only, i.e. a shared network).
//!
//! The offer and answer go by copied link: the host's lobby shares its offer as a link (`…?code=rtco…`); a guest
//! opens it (the game starts with the code, as Join by Code), and its lobby shares the answer as a link
//! (`…?code=rtca…`) for the host to Paste in its lobby (B526: the clipboard read, `web.rs`), or to open: that opens a
//! second tab on the host's device, which hands the answer to the game's tab (a `BroadcastChannel`, `web/index.html`)
//! and stops; pasting either code on the page (Ctrl+V) works too. The
//! host then shows a fresh offer for the next guest. Once connected it's the same lobby protocol as iroh's
//! (`net.rs`): JSON messages on a reliable, ordered data channel, rollback's datagrams on an unordered one with no
//! retransmits, both negotiated up front (ids 0 and 1) so the codes carry nothing but ICE and DTLS.
//!
//! A code is the description's essentials only (ICE ufrag and password, the DTLS fingerprint, the host candidates;
//! an answer also the offer's ufrag it answers), binary, base64url: ~100 bytes instead of a ~600-byte SDP, so the link
//! stays short. [`Sig::sdp`] builds a minimal description back from it.

use data_encoding::BASE64URL_NOPAD;

pub const OFFER: &str = "rtco";
pub const ANSWER: &str = "rtca";

/// A session description's essentials.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sig {
    pub ufrag: String,
    pub pwd: String,
    /// The DTLS certificate's SHA-256.
    pub fingerprint: [u8; 32],
    /// UDP host candidates: an IP address or an mDNS name (`<uuid>.local`, how browsers hide local addresses), port.
    pub cands: Vec<(String, u16)>,
    /// An answer: the ufrag of the offer it answers (empty in an offer).
    pub to: String,
}

impl Sig {
    /// The essentials of a browser's description (UDP host candidates of component 1 only).
    pub fn from_sdp(sdp: &str) -> Option<Sig> {
        let mut s = Sig::default();
        let mut fp = None;
        for l in sdp.lines().map(str::trim) {
            if let Some(v) = l.strip_prefix("a=ice-ufrag:") {
                s.ufrag = v.into();
            } else if let Some(v) = l.strip_prefix("a=ice-pwd:") {
                s.pwd = v.into();
            } else if let Some(v) = l.strip_prefix("a=fingerprint:sha-256 ") {
                let b: Vec<u8> = v.split(':').filter_map(|h| u8::from_str_radix(h, 16).ok()).collect();
                fp = b.try_into().ok();
            } else if let Some(v) = l.strip_prefix("a=candidate:") {
                let f: Vec<&str> = v.split_whitespace().collect();
                if f.len() >= 8 && f[1] == "1" && f[2].eq_ignore_ascii_case("udp") && f[7] == "host"
                    && let Ok(port) = f[5].parse()
                    && !s.cands.iter().any(|(a, p)| a == f[4] && *p == port)
                {
                    s.cands.push((f[4].into(), port));
                }
            }
        }
        s.fingerprint = fp?;
        (!s.ufrag.is_empty() && !s.pwd.is_empty()).then_some(s)
    }

    /// A minimal data-channel-only description from the essentials (`offer`: setup actpass, else active).
    pub fn sdp(&self, offer: bool) -> String {
        let fp: Vec<String> = self.fingerprint.iter().map(|b| format!("{b:02X}")).collect();
        let mut s = format!(
            "v=0\r\no=- 1 2 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\na=group:BUNDLE 0\r\nm=application 9 UDP/DTLS/SCTP webrtc-datachannel\r\n\
             c=IN IP4 0.0.0.0\r\na=mid:0\r\na=ice-ufrag:{}\r\na=ice-pwd:{}\r\na=fingerprint:sha-256 {}\r\na=setup:{}\r\n\
             a=sctp-port:5000\r\na=max-message-size:262144\r\n",
            self.ufrag,
            self.pwd,
            fp.join(":"),
            if offer { "actpass" } else { "active" }
        );
        for (i, (addr, port)) in self.cands.iter().enumerate() {
            s += &format!("a=candidate:{} 1 udp {} {addr} {port} typ host\r\n", i + 1, 2122260223 - i as u32);
        }
        s + "a=end-of-candidates\r\n"
    }

    /// The code: `kind` ([`OFFER`]/[`ANSWER`]) then the essentials, base64url.
    pub fn code(&self, kind: &str) -> String {
        let mut b = Vec::new();
        for t in [&self.ufrag, &self.pwd, &self.to] {
            b.push(t.len() as u8);
            b.extend_from_slice(t.as_bytes());
        }
        b.extend_from_slice(&self.fingerprint);
        for (addr, port) in &self.cands {
            match (addr.parse::<std::net::IpAddr>(), uuid(addr)) {
                (Ok(std::net::IpAddr::V4(ip)), _) => b.extend([4].iter().chain(&ip.octets())),
                (Ok(std::net::IpAddr::V6(ip)), _) => b.extend([6].iter().chain(&ip.octets())),
                (_, Some(u)) => b.extend(b"m".iter().chain(&u)),
                _ => b.extend([0, addr.len() as u8].iter().chain(addr.as_bytes())),
            }
            b.extend_from_slice(&port.to_be_bytes());
        }
        format!("{kind}{}", BASE64URL_NOPAD.encode(&b))
    }

    /// A code of `kind`, bare or in a link (`…?code=…`).
    pub fn parse(text: &str, kind: &str) -> Option<Sig> {
        let code = text.trim().rsplit("code=").next()?.split('&').next()?;
        let b = BASE64URL_NOPAD.decode(code.strip_prefix(kind)?.as_bytes()).ok()?;
        let mut r = b.as_slice();
        let mut take = |n: usize| -> Option<&[u8]> {
            let (a, rest) = r.split_at_checked(n)?;
            r = rest;
            Some(a)
        };
        let mut text = || -> Option<String> {
            let n = take(1)?[0] as usize;
            String::from_utf8(take(n)?.to_vec()).ok()
        };
        let (ufrag, pwd, to) = (text()?, text()?, text()?);
        let fingerprint = take(32)?.try_into().ok()?;
        let mut cands = Vec::new();
        while let Some(&[tag]) = take(1) {
            let addr = match tag {
                4 => std::net::Ipv4Addr::from(<[u8; 4]>::try_from(take(4)?).ok()?).to_string(),
                6 => std::net::Ipv6Addr::from(<[u8; 16]>::try_from(take(16)?).ok()?).to_string(),
                b'm' => {
                    let h: String = take(16)?.iter().map(|b| format!("{b:02x}")).collect();
                    format!("{}-{}-{}-{}-{}.local", &h[..8], &h[8..12], &h[12..16], &h[16..20], &h[20..])
                }
                0 => {
                    let n = take(1)?[0] as usize;
                    String::from_utf8(take(n)?.to_vec()).ok()?
                }
                _ => return None,
            };
            cands.push((addr, u16::from_be_bytes(take(2)?.try_into().ok()?)));
        }
        Some(Sig { ufrag, pwd, fingerprint, cands, to })
    }
}

/// `<uuid>.local` as the uuid's 16 bytes.
fn uuid(name: &str) -> Option<[u8; 16]> {
    let h: String = name.strip_suffix(".local")?.split('-').collect();
    if h.len() != 32 || name.len() != 42 || h.chars().any(|c| !c.is_ascii_hexdigit() || c.is_ascii_uppercase()) {
        return None;
    }
    (0..16).map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).ok()).collect::<Option<Vec<_>>>()?.try_into().ok()
}

pub fn is_offer(text: &str) -> bool {
    Sig::parse(text, OFFER).is_some()
}

/// B526: a guest's answer (its lobby's Copy Link shares it back to the host).
pub fn is_answer(text: &str) -> bool {
    Sig::parse(text, ANSWER).is_some()
}

thread_local! {
    /// The hosting lobby's inbox for answers (pasted, or handed over by a second tab).
    static ANSWERS: std::cell::RefCell<Option<tokio::sync::mpsc::UnboundedSender<String>>> = const { std::cell::RefCell::new(None) };
}

/// The answers handed to this page from now on (the host's lobby).
pub fn answers() -> tokio::sync::mpsc::UnboundedReceiver<String> {
    let (s, r) = tokio::sync::mpsc::unbounded_channel();
    ANSWERS.with(|a| *a.borrow_mut() = Some(s));
    r
}

/// `text` to the hosting lobby if it's an answer; true if it was one.
pub fn deliver(text: &str) -> bool {
    let yes = Sig::parse(text, ANSWER).is_some();
    if yes {
        ANSWERS.with(|a| a.borrow().as_ref().map(|s| s.send(text.into())));
    }
    yes
}

// ---------------------------------------------------------------- the browser's WebRTC

#[cfg(target_arch = "wasm32")]
pub use web::*;

#[cfg(target_arch = "wasm32")]
mod web {
    use super::super::{Conn, Inbox, Link, Msg};
    use super::{ANSWER, OFFER, Sig};
    use n0_future::time::{Duration, Instant, sleep};
    use tokio::sync::mpsc as amp;
    use wasm_bindgen::{JsCast, JsValue, closure::Closure};
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{
        MessageEvent, RtcDataChannel, RtcDataChannelInit, RtcDataChannelState, RtcDataChannelType, RtcIceGatheringState, RtcPeerConnection,
        RtcPeerConnectionState, RtcSdpType, RtcSessionDescriptionInit,
    };

    /// One WebRTC connection: messages on channel 0, datagrams on channel 1.
    #[derive(Clone)]
    pub struct Peer {
        pc: RtcPeerConnection,
        msgs: RtcDataChannel,
        dgrams: RtcDataChannel,
    }

    fn js(e: JsValue) -> String {
        e.as_string().or_else(|| e.dyn_ref::<js_sys::Error>().map(|e| String::from(e.message()))).unwrap_or_else(|| format!("{e:?}"))
    }

    impl Peer {
        /// A connection with its channels and their handlers (set at once: a message can arrive the moment a channel
        /// opens).
        fn new() -> Result<(Peer, Inbox), String> {
            // no ICE servers: host candidates only, this network
            let pc = RtcPeerConnection::new().map_err(js)?;
            let ch = |id: u16, label: &str, reliable: bool| {
                let init = RtcDataChannelInit::new();
                init.set_negotiated(true);
                init.set_id(id);
                if !reliable {
                    init.set_ordered(false);
                    init.set_max_retransmits(0);
                }
                pc.create_data_channel_with_data_channel_dict(label, &init)
            };
            let (msgs, dgrams) = (ch(0, "msgs", true), ch(1, "dgrams", false));
            dgrams.set_binary_type(RtcDataChannelType::Arraybuffer);
            let (mt, msgs_rx) = amp::unbounded_channel();
            let (dt, dgrams_rx) = amp::unbounded_channel();
            // ponytail: the handlers are leaked (a few closures a connection); keep them on `Peer` if lobbies churn
            let m = mt.clone();
            let on_msg = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
                let m2 = e.data().as_string().ok_or("not text".to_string()).and_then(|s| serde_json::from_str::<Msg>(&s).map_err(|e| e.to_string()));
                let _ = m.send(m2);
            });
            msgs.set_onmessage(Some(on_msg.as_ref().unchecked_ref()));
            on_msg.forget();
            let m = mt.clone();
            let on_close = Closure::<dyn FnMut()>::new(move || {
                let _ = m.send(Err("closed".into()));
            });
            msgs.set_onclose(Some(on_close.as_ref().unchecked_ref()));
            on_close.forget();
            let p = pc.clone();
            let on_state = Closure::<dyn FnMut()>::new(move || {
                if p.connection_state() == RtcPeerConnectionState::Failed {
                    let _ = mt.send(Err("connection lost".into()));
                }
            });
            pc.set_onconnectionstatechange(Some(on_state.as_ref().unchecked_ref()));
            on_state.forget();
            let on_dgram = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
                let _ = dt.send(js_sys::Uint8Array::new(&e.data()).to_vec());
            });
            dgrams.set_onmessage(Some(on_dgram.as_ref().unchecked_ref()));
            on_dgram.forget();
            Ok((Peer { pc, msgs, dgrams }, Inbox { msgs: msgs_rx, dgrams: dgrams_rx }))
        }

        pub fn send_datagram(&self, d: &[u8]) {
            let _ = self.dgrams.send_with_u8_array(d);
        }

        pub fn close(&self) {
            self.pc.close();
        }

        /// Our description (`made`: the offer/answer promise) set, once ICE gathering is done (host candidates: well
        /// under a second; 3 s at most).
        async fn local(&self, made: js_sys::Promise, ty: RtcSdpType) -> Result<Sig, String> {
            let d = JsFuture::from(made).await.map_err(js)?;
            let sdp = js_sys::Reflect::get(&d, &"sdp".into()).map_err(js)?.as_string().unwrap_or_default();
            let init = RtcSessionDescriptionInit::new(ty);
            init.set_sdp(&sdp);
            JsFuture::from(self.pc.set_local_description(&init)).await.map_err(js)?;
            let t = Instant::now();
            while self.pc.ice_gathering_state() != RtcIceGatheringState::Complete && t.elapsed() < Duration::from_secs(3) {
                sleep(Duration::from_millis(50)).await;
            }
            let sdp = self.pc.local_description().ok_or("no local description")?.sdp();
            let sig = Sig::from_sdp(&sdp).ok_or("this browser's description has no ICE/DTLS details")?;
            if sig.cands.is_empty() {
                return Err("No network address: join a Wi-Fi network (or a phone's hotspot) first.".into());
            }
            Ok(sig)
        }

        async fn remote(&self, sig: &Sig, ty: RtcSdpType) -> Result<(), String> {
            let init = RtcSessionDescriptionInit::new(ty);
            init.set_sdp(&sig.sdp(ty == RtcSdpType::Offer));
            JsFuture::from(self.pc.set_remote_description(&init)).await.map_err(js).map(drop)
        }

        /// Both channels open: the connection, its message link and arrivals.
        async fn open(self, inbox: Inbox) -> Result<(Conn, Link, Inbox), String> {
            loop {
                if self.msgs.ready_state() == RtcDataChannelState::Open && self.dgrams.ready_state() == RtcDataChannelState::Open {
                    let link = link(self.msgs.clone());
                    return Ok((Conn::Rtc(self), link, inbox));
                }
                if matches!(self.pc.connection_state(), RtcPeerConnectionState::Failed | RtcPeerConnectionState::Closed) {
                    self.close();
                    return Err("Couldn't connect: are both on the same network?".into());
                }
                sleep(Duration::from_millis(50)).await;
            }
        }
    }

    /// Messages out on the reliable channel, each after [`super::super::LAG_MS`], in order.
    fn link(dc: RtcDataChannel) -> Link {
        let (s, mut r) = amp::unbounded_channel::<(Instant, Msg)>();
        n0_future::task::spawn(async move {
            while let Some((due, m)) = r.recv().await {
                n0_future::time::sleep_until(due).await;
                if dc.send_with_str(&serde_json::to_string(&m).unwrap()).is_err() {
                    break;
                }
            }
        });
        Link(s)
    }

    /// `code` as a link to this page, so a phone's camera opens the game with it.
    fn url(code: &str) -> String {
        let l = web_sys::window().unwrap().location();
        format!("{}{}?code={code}", l.origin().unwrap_or_default(), l.pathname().unwrap_or_default())
    }

    /// The host's side: an offer waiting for its answer.
    pub struct Offer {
        peer: Peer,
        inbox: Inbox,
        sig: Sig,
    }

    /// A fresh offer and its code (a link).
    pub async fn offer() -> Result<(Offer, String), String> {
        let (peer, inbox) = Peer::new()?;
        let sig = peer.local(peer.pc.create_offer(), RtcSdpType::Offer).await?;
        let code = url(&sig.code(OFFER));
        Ok((Offer { peer, inbox, sig }, code))
    }

    impl Offer {
        /// `code` answers this offer (not an older one).
        pub fn answers(&self, code: &str) -> bool {
            Sig::parse(code, ANSWER).is_some_and(|a| a.to == self.sig.ufrag)
        }

        /// Connect with the guest's answer (30 s at most): the guest's peer name (its ufrag) and the connection.
        pub async fn accept(self, code: &str) -> Result<(String, Conn, Link, Inbox), String> {
            let a = Sig::parse(code, ANSWER).ok_or("not an answer code")?;
            self.peer.remote(&a, RtcSdpType::Answer).await?;
            let pc = self.peer.pc.clone();
            match n0_future::time::timeout(Duration::from_secs(30), self.peer.open(self.inbox)).await {
                Ok(Ok((c, l, i))) => Ok((a.ufrag, c, l, i)),
                Ok(Err(e)) => Err(e),
                Err(_) => {
                    pc.close();
                    Err("A player's connection timed out: are both on the same network?".into())
                }
            }
        }
    }

    /// The guest's side: answered, waiting for the host to take the answer.
    pub struct Pending {
        peer: Peer,
        inbox: Inbox,
        me: String,
    }

    /// Answer the offer in `code`: the pending connection and the answer's code (a link).
    pub async fn answer(code: &str) -> Result<(Pending, String), String> {
        let o = Sig::parse(code, OFFER).ok_or("not a LAN lobby code")?;
        let (peer, inbox) = Peer::new()?;
        peer.remote(&o, RtcSdpType::Offer).await?;
        let mut sig = peer.local(peer.pc.create_answer(), RtcSdpType::Answer).await?;
        sig.to = o.ufrag;
        let code = url(&sig.code(ANSWER));
        Ok((Pending { peer, inbox, me: sig.ufrag }, code))
    }

    impl Pending {
        /// This guest's peer name, as the host will know it (the answer's ufrag).
        pub fn me(&self) -> String {
            self.me.clone()
        }

        /// Connected once the host has taken the answer.
        pub async fn open(self) -> Result<(Conn, Link, Inbox), String> {
            self.peer.open(self.inbox).await
        }
    }
}

// natively there's no WebRTC (LAN is iroh's, by mDNS): the types exist so the lobby code is the same
#[cfg(not(target_arch = "wasm32"))]
pub enum Offer {}

#[cfg(not(target_arch = "wasm32"))]
pub enum Pending {}

#[cfg(not(target_arch = "wasm32"))]
const BROWSER: &str = "LAN codes from a browser (rtco…) only work in the browser build; on this machine use Find or the host's code.";

#[cfg(not(target_arch = "wasm32"))]
impl Offer {
    pub fn answers(&self, _: &str) -> bool {
        match *self {}
    }

    pub async fn accept(self, _: &str) -> Result<(String, super::Conn, super::Link, super::Inbox), String> {
        match self {}
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Pending {
    pub fn me(&self) -> String {
        match *self {}
    }

    pub async fn open(self) -> Result<(super::Conn, super::Link, super::Inbox), String> {
        match self {}
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn offer() -> Result<(Offer, String), String> {
    Err(BROWSER.into())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn answer(_: &str) -> Result<(Pending, String), String> {
    Err(BROWSER.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Chromium data-channel offer (made up addresses and keys, real shape).
    const CHROME: &str = "v=0\r\no=- 4611731400430051336 2 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\na=group:BUNDLE 0\r\n\
        a=extmap-allow-mixed\r\na=msid-semantic: WMS\r\nm=application 9 UDP/DTLS/SCTP webrtc-datachannel\r\nc=IN IP4 0.0.0.0\r\n\
        a=candidate:1467250027 1 udp 2122260223 0b3c47a1-6d0e-4b8a-9c2f-5e1d2a3b4c5d.local 54400 typ host generation 0 network-cost 999\r\n\
        a=candidate:1467250028 1 udp 2122194687 192.168.1.20 54401 typ host generation 0\r\n\
        a=candidate:435653019 1 tcp 1518280447 192.168.1.20 9 typ host tcptype active generation 0\r\n\
        a=candidate:1 1 udp 1686052607 203.0.113.9 54400 typ srflx raddr 192.168.1.20 rport 54400\r\n\
        a=candidate:2 1 udp 2122129151 fe80::1 54402 typ host\r\n\
        a=ice-ufrag:x+/Q\r\na=ice-pwd:a/b+cdefghijklmnopqrstuvw\r\na=ice-options:trickle\r\n\
        a=fingerprint:sha-256 0A:1B:2C:3D:4E:5F:60:71:82:93:A4:B5:C6:D7:E8:F9:0A:1B:2C:3D:4E:5F:60:71:82:93:A4:B5:C6:D7:E8:F9\r\n\
        a=setup:actpass\r\na=mid:0\r\na=sctp-port:5000\r\na=max-message-size:262144\r\n";

    #[test]
    fn code_round_trips() {
        let mut s = Sig::from_sdp(CHROME).unwrap();
        assert_eq!((s.ufrag.as_str(), s.pwd.as_str(), s.fingerprint[0], s.fingerprint[31]), ("x+/Q", "a/b+cdefghijklmnopqrstuvw", 0x0a, 0xf9));
        // UDP host candidates only: no TCP, no server-reflexive
        let addrs: Vec<&str> = s.cands.iter().map(|(a, _)| a.as_str()).collect();
        assert_eq!(addrs, ["0b3c47a1-6d0e-4b8a-9c2f-5e1d2a3b4c5d.local", "192.168.1.20", "fe80::1"]);
        s.to = "abcd".into();
        let code = s.code(ANSWER);
        assert!(code.len() < 160, "{} chars", code.len());
        assert_eq!(Sig::parse(&code, ANSWER), Some(s.clone()));
        // in a link, and not as the other kind
        assert_eq!(Sig::parse(&format!("https://x.org/hst/?code={code}"), ANSWER), Some(s.clone()));
        assert_eq!(Sig::parse(&code, OFFER), None);
        assert!(!is_offer(&code));
        // the rebuilt description says the same
        let back = Sig::from_sdp(&s.sdp(false)).unwrap();
        assert_eq!(back, Sig { to: String::new(), ..s });
        assert!(Sig::parse("rtcoAAAA", OFFER).is_none());
        assert!(Sig::parse("endpointabc", OFFER).is_none());
    }

    #[test]
    fn answers_reach_the_host_only_when_answers() {
        let mut r = answers();
        let a = Sig::from_sdp(CHROME).unwrap().code(ANSWER);
        assert!(!deliver(&Sig::from_sdp(CHROME).unwrap().code(OFFER)));
        assert!(deliver(&a));
        assert_eq!(r.try_recv().ok(), Some(a));
    }
}
