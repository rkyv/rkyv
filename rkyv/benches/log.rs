use benchlib::{bench_dataset, generate_vec, Generate, Rng};
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct Address {
    pub x0: u8,
    pub x1: u8,
    pub x2: u8,
    pub x3: u8,
}

impl Generate for Address {
    fn generate<R: Rng>(rand: &mut R) -> Self {
        Self {
            x0: rand.gen_range(0..=255),
            x1: rand.gen_range(0..=255),
            x2: rand.gen_range(0..=255),
            x3: rand.gen_range(0..=255),
        }
    }
}

#[derive(Archive, Serialize, Deserialize, Clone, PartialEq)]
pub struct Log {
    pub address: Address,
    pub identity: String,
    pub userid: String,
    pub date: String,
    pub request: String,
    pub code: u16,
    pub size: u64,
}

impl Generate for Log {
    fn generate<R: Rng>(rand: &mut R) -> Self {
        const USERID: [&str; 9] = [
            "-", "alice", "bob", "carmen", "david", "eric", "frank", "george",
            "harry",
        ];
        const MONTHS: [&str; 12] = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep",
            "Oct", "Nov", "Dec",
        ];
        const TIMEZONE: [&str; 25] = [
            "-1200", "-1100", "-1000", "-0900", "-0800", "-0700", "-0600",
            "-0500", "-0400", "-0300", "-0200", "-0100", "+0000", "+0100",
            "+0200", "+0300", "+0400", "+0500", "+0600", "+0700", "+0800",
            "+0900", "+1000", "+1100", "+1200",
        ];
        let date = format!(
            "{}/{}/{}:{}:{}:{} {}",
            rand.gen_range(1..=28),
            MONTHS[rand.gen_range(0..12)],
            rand.gen_range(1970..=2021),
            rand.gen_range(0..24),
            rand.gen_range(0..60),
            rand.gen_range(0..60),
            TIMEZONE[rand.gen_range(0..25)],
        );
        const CODES: [u16; 63] = [
            100, 101, 102, 103, 200, 201, 202, 203, 204, 205, 206, 207, 208,
            226, 300, 301, 302, 303, 304, 305, 306, 307, 308, 400, 401, 402,
            403, 404, 405, 406, 407, 408, 409, 410, 411, 412, 413, 414, 415,
            416, 417, 418, 421, 422, 423, 424, 425, 426, 428, 429, 431, 451,
            500, 501, 502, 503, 504, 505, 506, 507, 508, 510, 511,
        ];
        const METHODS: [&str; 5] = ["GET", "POST", "PUT", "UPDATE", "DELETE"];
        const ROUTES: [&str; 7] = [
            "/favicon.ico",
            "/css/index.css",
            "/css/font-awsome.min.css",
            "/img/logo-full.svg",
            "/img/splash.jpg",
            "/api/login",
            "/api/logout",
        ];
        const PROTOCOLS: [&str; 4] =
            ["HTTP/1.0", "HTTP/1.1", "HTTP/2", "HTTP/3"];
        let request = format!(
            "{} {} {}",
            METHODS[rand.gen_range(0..5)],
            ROUTES[rand.gen_range(0..7)],
            PROTOCOLS[rand.gen_range(0..4)],
        );
        Self {
            address: Address::generate(rand),
            identity: "-".into(),
            userid: USERID[rand.gen_range(0..USERID.len())].into(),
            date,
            request,
            code: CODES[rand.gen_range(0..CODES.len())],
            size: rand.gen_range(0..100_000_000),
        }
    }
}

#[derive(Archive, Serialize, Deserialize, Clone, PartialEq)]
pub struct Logs {
    pub logs: Vec<Log>,
}

pub fn generate_logs() -> Logs {
    let mut rng = benchlib::rng();

    const LOGS: usize = 10_000;
    Logs {
        logs: generate_vec::<_, Log>(&mut rng, LOGS..LOGS + 1),
    }
}

bench_dataset!(Logs = generate_logs());
