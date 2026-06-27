pub mod listen_moe;

pub const RADIO_STATIONS: &[(&str, &str, bool)] = &[
    ("1) J-Pop (LISTEN.moe)", "https://listen.moe/stream", true),
    (
        "2) K-Pop (LISTEN.moe)",
        "https://listen.moe/kpop/stream",
        true,
    ),
    (
        "3) Vocaloid (Vocaloid Radio)",
        "http://curiosity.shoutca.st:8019/stream",
        false,
    ),
    (
        "4) Lofi Hip Hop (Lofi 24/7)",
        "http://usa9.fastcast4u.com/proxy/jamz?mp=/1",
        false,
    ),
    (
        "5) Ambient (SomaFM Groove Salad)",
        "https://ice1.somafm.com/groovesalad-256-mp3",
        false,
    ),
    (
        "6) Metal (SomaFM Metal Detector)",
        "https://ice1.somafm.com/metal-128-mp3",
        false,
    ),
    ("q) Back", "", false),
];
