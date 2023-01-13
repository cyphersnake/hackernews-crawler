use std::{num::NonZeroUsize, time::Duration};

use anyhow::Result;
use voyager::{
    scraper::Selector, Collector, Crawler, CrawlerConfig, RequestDelay, Response, Scraper,
};

use hackernews_crawler::hackernews_core::{DateTime, Post as Entry, PostId};

#[derive(Debug)]
pub enum HackernewsState {
    Page {
        snapshot_time: DateTime,
        page: usize,
    },
    Post {
        snapshot_time: DateTime,
        post_id: PostId,
        page: usize,
    },
}

#[derive(Debug, Clone)]
pub struct HackernewsScraper {
    post_selector: Selector,
    author_selector: Selector,
    publication_moment_selector: Selector,
    title_selector: Selector,
    max_page: NonZeroUsize,
}

impl Default for HackernewsScraper {
    fn default() -> Self {
        Self {
            post_selector: Selector::parse("#hnmain tr.athing").unwrap(),
            author_selector: Selector::parse("a.hnuser").unwrap(),
            publication_moment_selector: Selector::parse("span.age").unwrap(),
            title_selector: Selector::parse("td.title a").unwrap(),
            max_page: NonZeroUsize::new(10).unwrap(),
        }
    }
}

impl HackernewsScraper {
    pub fn new_collector(&self, delay: Duration) -> Collector<Self> {
        let config = CrawlerConfig::default()
            .allow_domain_with_delay("news.ycombinator.com", RequestDelay::Fixed(delay));

        let mut collector = Collector::new(self.clone(), config);
        collector.crawler_mut().visit_with_state(
            "https://news.ycombinator.com/news",
            HackernewsState::Page {
                page: 1,
                snapshot_time: chrono::Local::now().naive_utc(),
            },
        );

        collector
    }
}

pub trait HackernewsCrawler {
    fn visit_page(&mut self, page: usize, snapshot_time: DateTime);
    fn visit_post(&mut self, post_id: PostId, page: usize, snapshot_time: DateTime);
}
impl HackernewsCrawler for Crawler<HackernewsScraper> {
    fn visit_page(&mut self, page: usize, snapshot_time: DateTime) {
        self.visit_with_state(
            &format!("https://news.ycombinator.com/news?p={page}"),
            HackernewsState::Page {
                page,
                snapshot_time,
            },
        );
    }

    fn visit_post(&mut self, post_id: PostId, page: usize, snapshot_time: DateTime) {
        self.visit_with_state(
            &format!("https://news.ycombinator.com/item?id={post_id}"),
            HackernewsState::Post {
                post_id,
                page,
                snapshot_time,
            },
        )
    }
}

impl HackernewsScraper {
    // This function is implemented to be able to replace the crawler to trait object
    fn scrape_internal(
        &mut self,
        response: Response<HackernewsState>,
        crawler: &mut impl HackernewsCrawler,
    ) -> Result<Option<(usize, Entry)>> {
        let html = response.html();

        Ok(match response.state {
            Some(HackernewsState::Page {
                page,
                snapshot_time,
            }) => {
                tracing::info!("start visit {page} page");
                for id in html
                    .select(&self.post_selector)
                    .filter_map(|el| el.value().attr("id"))
                {
                    tracing::info!("let's visit post with {id}");
                    crawler.visit_post(
                        id.parse().expect("Error handling needed here"),
                        page,
                        snapshot_time,
                    );
                }

                if page < self.max_page.get() {
                    tracing::info!("let's visit {page} page", page = page + 1);
                    crawler.visit_page(page + 1, snapshot_time);
                } else {
                    tracing::info!("scrapping ended at {page}");
                }
                None
            }
            Some(HackernewsState::Post {
                post_id,
                page,
                snapshot_time,
            }) => {
                tracing::info!(
                    "visited post {post_id} at {page} with snapshot time: {snapshot_time}"
                );
                let el_title = html.select(&self.title_selector).next().unwrap();
                let author = match html
                    .select(&self.author_selector)
                    .map(|el| el.inner_html())
                    .next()
                {
                    Some(author) => author,
                    None => {
                        tracing::warn!("In {post_id} can't parse author");
                        "unknown".to_owned()
                    }
                };

                let publication_moment = match html
                    .select(&self.publication_moment_selector)
                    .map(|el| el.value().attr("title"))
                    .next()
                    .flatten()
                    .and_then(|publication_moment| {
                        tracing::debug!("publication moment raw {publication_moment}");
                        DateTime::parse_from_str(publication_moment.trim(), "%Y-%m-%dT%H:%M:%S")
                            .ok()
                    }) {
                    Some(moment) => moment,
                    None => {
                        tracing::error!("Ignore {post_id} because can't parse publication moment");
                        return Ok(None);
                    }
                };

                Some((
                    page,
                    Entry {
                        post_id,
                        author,
                        url: response.response_url.to_string(),
                        link: el_title.value().attr("href").map(str::to_string),
                        title: el_title.inner_html(),
                        publication_moment,
                        last_snapshot_moment: snapshot_time,
                    },
                ))
            }
            None => None,
        })
    }
}

impl Scraper for HackernewsScraper {
    type Output = (usize, Entry);
    type State = HackernewsState;

    fn scrape(
        &mut self,
        response: Response<Self::State>,
        crawler: &mut Crawler<Self>,
    ) -> Result<Option<Self::Output>> {
        self.scrape_internal(response, crawler)
    }
}

#[cfg(test)]
mod tests {
    use reqwest::{header::HeaderMap, StatusCode};

    use super::*;
    struct CrawlerMock {
        expected_visits: Vec<HackernewsState>,
    }
    impl HackernewsCrawler for CrawlerMock {
        fn visit_page(&mut self, expected_page: usize, expected_snapshot_time: DateTime) {
            match self.expected_visits.pop().expect("visit not exptected") {
                HackernewsState::Page {
                    page,
                    snapshot_time,
                } => {
                    assert_eq!(expected_page, page);
                    assert_eq!(expected_snapshot_time, snapshot_time);
                }
                HackernewsState::Post { .. } => {
                    panic!("Expected page, not post visit")
                }
            }
        }

        fn visit_post(
            &mut self,
            expected_post_id: PostId,
            expected_page: usize,
            expected_snapshot_time: DateTime,
        ) {
            match self.expected_visits.pop().expect("visit not expected") {
                HackernewsState::Page { .. } => panic!("Exptected page, not post"),
                HackernewsState::Post {
                    snapshot_time,
                    post_id,
                    page,
                } => {
                    assert_eq!(expected_post_id, post_id);
                    assert_eq!(expected_page, page);
                    assert_eq!(expected_snapshot_time, snapshot_time);
                }
            }
        }
    }

    #[test]
    fn test_visit_news_page() {
        let snapshot_time = chrono::Local::now().naive_utc();
        // TODO Add parsing of next page
        let mut mock = CrawlerMock {
            expected_visits: [
                34388962, 34388369, 34387409, 34388866, 34388826, 34389037, 34388095, 34388670,
                34388773, 34388985, 34382212, 34387407, 34384825, 34386309, 34386570, 34386052,
                34384941, 34385766, 34384719, 34384767, 34386929, 34386443, 34387081, 34386876,
                34385223, 34387834, 34384681, 34383529, 34376781, 34386017,
            ]
            .into_iter()
            .rev()
            .map(|post_id| HackernewsState::Post {
                snapshot_time,
                post_id,
                page: 1,
            })
            .collect(),
        };

        HackernewsScraper {
            max_page: NonZeroUsize::new(1).unwrap(),
            ..Default::default()
        }
        .scrape_internal(
            Response {
                depth: 0,
                request_url: "https://news.ycombinator.com/".parse().unwrap(),
                response_url: "https://news.ycombinator.com/".parse().unwrap(),
                response_status: StatusCode::OK,
                response_headers: HeaderMap::default(),
                text: include_str!("../../fixtures/first_page.html").to_string(),
                state: Some(HackernewsState::Page {
                    page: 1,
                    snapshot_time,
                }),
            },
            &mut mock,
        )
        .unwrap();
    }
}
