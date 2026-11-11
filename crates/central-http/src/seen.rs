//! What the browser listener sees of a request that asks for a registration
//! challenge (`docs/registration-abuse.md` §"Hardened"): a few plain facts
//! for the identity service, which makes the challenge harder for a request
//! that looks automated. The headers' texts stay here: none is passed on,
//! kept or logged, and no fact ever refuses a request.
use axum::http::{HeaderMap, header};
use central_identity::{Agent, Seen};

/// User agents that say what they are: headless browsers, tools, libraries
/// and crawlers (`Googlebot/2.1`, `bingbot/2.0`; a crawler's contact link
/// `+http`), as the Impressum's service knows them. "bot" alone would also
/// be a phone maker's name.
const MARKERS: [&str; 18] = [
    "headless",
    "bot/",
    "+http",
    "crawl",
    "spider",
    "curl",
    "wget",
    "python",
    "go-http",
    "java/",
    "okhttp",
    "libwww",
    "scrapy",
    "httpclient",
    "phantomjs",
    "selenium",
    "puppeteer",
    "playwright",
];

/// Every value of the `User-Agent`, lowercased; empty without one.
fn agent(headers: &HeaderMap) -> Vec<u8> {
    let mut text = Vec::new();
    for value in headers.get_all(header::USER_AGENT) {
        text.extend_from_slice(value.as_bytes().trim_ascii());
        text.push(b' ');
    }
    text.make_ascii_lowercase();
    text
}

fn names(agent: &[u8], marker: &str) -> bool {
    agent
        .windows(marker.len())
        .any(|part| part == marker.as_bytes())
}

/// The facts of one request's headers, with `frequent` as the listener
/// counted its address's challenges (0 to 3).
pub(crate) fn glance(headers: &HeaderMap, frequent: u8) -> Seen {
    let text = agent(headers);
    let agent = if text.trim_ascii().is_empty() {
        Agent::Missing
    } else if MARKERS.iter().any(|marker| names(&text, marker)) {
        Agent::Tool
    } else {
        Agent::Browser
    };
    Seen {
        no_language: !headers
            .get_all(header::ACCEPT_LANGUAGE)
            .iter()
            .any(|value| !value.as_bytes().trim_ascii().is_empty()),
        // One that is sent and is not `same-origin` was refused before this.
        no_fetch_site: !headers.contains_key("sec-fetch-site"),
        agent,
        frequent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    const FIREFOX: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:130.0) Gecko/20100101 Firefox/130.0";
    const SAFARI: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_5 like Mac OS X) \
        AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.5 Mobile/15E148 Safari/604.1";
    const HEADLESS: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like \
        Gecko) HeadlessChrome/126.0.0.0 Safari/537.36";

    fn browser(agent: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::USER_AGENT, HeaderValue::from_str(agent).unwrap());
        headers.insert(
            header::ACCEPT_LANGUAGE,
            HeaderValue::from_static("de-DE,de;q=0.9,en;q=0.8"),
        );
        headers.insert("sec-fetch-site", HeaderValue::from_static("same-origin"));
        headers
    }

    #[test]
    fn a_person_s_browser_shows_nothing() {
        for agent in [
            FIREFOX,
            SAFARI,
            // a phone maker's name is no robot's
            "Mozilla/5.0 (Linux; Android 14; Cubot P80) AppleWebKit/537.36 Chrome/126 Mobile",
        ] {
            assert_eq!(glance(&browser(agent), 0), Seen::default(), "{agent}");
        }
        assert_eq!(glance(&browser(FIREFOX), 2).frequent, 2);
    }

    #[test]
    fn each_missing_header_is_its_own_fact() {
        let mut headers = browser(FIREFOX);
        headers.remove(header::ACCEPT_LANGUAGE);
        let seen = glance(&headers, 0);
        assert!(seen.no_language);
        assert_eq!(
            seen,
            Seen {
                no_language: true,
                ..Seen::default()
            }
        );
        let mut headers = browser(FIREFOX);
        headers.insert(header::ACCEPT_LANGUAGE, HeaderValue::from_static(" "));
        assert!(glance(&headers, 0).no_language, "an empty one is none");
        let mut headers = browser(FIREFOX);
        headers.remove("sec-fetch-site");
        assert_eq!(
            glance(&headers, 0),
            Seen {
                no_fetch_site: true,
                ..Seen::default()
            }
        );
        let mut headers = browser(FIREFOX);
        headers.remove(header::USER_AGENT);
        assert_eq!(
            glance(&headers, 0),
            Seen {
                agent: Agent::Missing,
                ..Seen::default()
            }
        );
        headers.insert(header::USER_AGENT, HeaderValue::from_static("  "));
        assert_eq!(glance(&headers, 0).agent, Agent::Missing);
        assert_eq!(
            glance(&HeaderMap::new(), 3),
            Seen {
                no_language: true,
                no_fetch_site: true,
                agent: Agent::Missing,
                frequent: 3,
            }
        );
    }

    #[test]
    fn a_tool_a_crawler_and_a_headless_browser_say_what_they_are() {
        for agent in [
            HEADLESS,
            "curl/8.7.1",
            "Wget/1.21",
            "python-requests/2.32.3",
            "Python-urllib/3.12",
            "Go-http-client/2.0",
            "Java/21.0.1",
            "okhttp/4.12.0",
            "Scrapy/2.11 (+https://scrapy.org)",
            "Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)",
            "Mozilla/5.0 (compatible; bingbot/2.0; +http://www.bing.com/bingbot.htm)",
            "Apache-HttpClient/5.3 (Java/17)",
            "Mozilla/5.0 (Unknown; Linux x86_64) AppleWebKit/538.1 PhantomJS/2.1.1 Safari/538.1",
            "Mozilla/5.0 Playwright/1.45",
        ] {
            assert_eq!(glance(&browser(agent), 0).agent, Agent::Tool, "{agent}");
        }
        // a second value does not hide the first
        let mut headers = browser(FIREFOX);
        headers.append(header::USER_AGENT, HeaderValue::from_static("curl/8.7.1"));
        assert_eq!(glance(&headers, 0).agent, Agent::Tool);
        // bytes that are no text are read as bytes
        let mut headers = browser(FIREFOX);
        headers.insert(
            header::USER_AGENT,
            HeaderValue::from_bytes(b"\xff\xfe cURL/8").unwrap(),
        );
        assert_eq!(glance(&headers, 0).agent, Agent::Tool);
    }
}
