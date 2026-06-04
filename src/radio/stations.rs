#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StationKind {
    Direct,
    YouTube,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Station {
    pub code: &'static str,
    pub description: &'static str,
    pub url: &'static str,
    pub kind: StationKind,
}

const STATIONS: &[Station] = &[
    Station {
        code: "atma",
        description: "atma.fm Channel 1 - Ambient and experimental electroacoustic music",
        url: "https://atma.fm/channel1",
        kind: StationKind::Direct,
    },
    Station {
        code: "atm2",
        description: "atma.fm Channel 2 - Darkwave, dark ambient, and neoclassical/gothic music",
        url: "https://atma.fm/channel2",
        kind: StationKind::Direct,
    },
    Station {
        code: "ssom",
        description: "SomaFM Space Station Soma - Spaced-out ambient and mid-tempo electronica",
        url: "https://ice5.somafm.com/spacestation-128-mp3",
        kind: StationKind::Direct,
    },
    Station {
        code: "beat",
        description: "SomaFM Beat Blender - Eclectic downtempo and electronic music",
        url: "https://ice5.somafm.com/beatblender-128-mp3",
        kind: StationKind::Direct,
    },
    Station {
        code: "grve",
        description: "SomaFM Groove Salad - Listener-supported downtempo and chill electronic music",
        url: "https://ice5.somafm.com/groovesalad-128-mp3",
        kind: StationKind::Direct,
    },
    Station {
        code: "nood",
        description: "Noods Radio - Music-heavy community radio from Bristol",
        url: "https://noods-radio.radiocult.fm/stream",
        kind: StationKind::Direct,
    },
    Station {
        code: "drmm",
        description: "Intergalactic FM Dream Machine - Experimental music from The Hague",
        url: "https://radio.intergalactic.fm/3A",
        kind: StationKind::Direct,
    },
    Station {
        code: "9128",
        description: "9128.live - Curated ambient/drone stream with zero talk",
        url: "https://streams.radio.co/s0aa1e6f4a/listen",
        kind: StationKind::Direct,
    },
    Station {
        code: "arab",
        description: "Arab Mix FM - Arabic music stream replacement for Radio Alhara",
        url: "https://stream.zeno.fm/na3vpvn10qruv.acc",
        kind: StationKind::Direct,
    },
    Station {
        code: "ytlf",
        description: "Lofi Girl - lofi hip hop radio - beats to relax/study to",
        url: "https://www.youtube.com/watch?v=X4VbdwhkE10",
        kind: StationKind::YouTube,
    },
];

pub const fn all() -> &'static [Station] {
    STATIONS
}

pub fn find(code: &str) -> Option<&'static Station> {
    STATIONS.iter().find(|station| station.code == code)
}

pub fn format_listing() -> String {
    all()
        .iter()
        .map(|station| format!("{:<4}  {}", station.code, station.description))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_ruv_codes() {
        let codes: Vec<_> = all().iter().map(|station| station.code).collect();
        assert_eq!(
            codes,
            [
                "atma", "atm2", "ssom", "beat", "grve", "nood", "drmm", "9128", "arab", "ytlf",
            ]
        );
    }

    #[test]
    fn finds_station_by_exact_code() {
        assert_eq!(find("atma").unwrap().url, "https://atma.fm/channel1");
        assert_eq!(find("atm2").unwrap().url, "https://atma.fm/channel2");
        assert!(find("ATMA").is_none());
    }

    #[test]
    fn finds_youtube_station_by_code() {
        let station = find("ytlf").expect("ytlf station should exist");
        assert_eq!(station.kind, StationKind::YouTube);
    }

    #[test]
    fn listing_matches_ruv_plain_format() {
        let listing = format_listing();
        assert!(listing.starts_with("atma  atma.fm Channel 1"));
        assert!(listing.contains(
            "\natm2  atma.fm Channel 2 - Darkwave, dark ambient, and neoclassical/gothic music"
        ));
        assert!(listing.contains("\nssom  SomaFM Space Station Soma"));
        assert!(
            listing.ends_with("ytlf  Lofi Girl - lofi hip hop radio - beats to relax/study to")
        );
        assert_eq!(listing.lines().count(), all().len());
    }
}
