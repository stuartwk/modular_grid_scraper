use futures::StreamExt;
// use log::{info, warn};
use voyager::{
    scraper::{ElementRef, Selector},
    Collector, Crawler, CrawlerConfig, RequestDelay, Response, Scraper,
};

// Declare your scraper, with all the selectors etc.
struct ModularGridScraper {
    module_selector: Selector,
    module_name_selector: Selector,
    module_manufacturer_selector: Selector,
    width_selector: Selector,
    depth_selector: Selector,
    positive_12_selector: Selector,
    negative_12_selector: Selector,
    positive_5_selector: Selector,
    max_page: usize,
}

#[derive(Debug)]
struct CurrentDraw {
    positive_12: Option<u8>,
    negative_12: Option<u8>,
    positive_5: Option<u8>,
}

impl ModularGridScraper {
    fn parse_width(&self, width_elem: ElementRef) -> Option<u8> {
        let width_inner = width_elem.inner_html();
        let mut split_width = width_inner.split_whitespace();
        let width = split_width.next().unwrap();

        match width.parse::<u8>() {
            Ok(width) => Some(width),
            Err(_) => None,
        }
    }

    fn parse_depth(&self, depth_elem: ElementRef) -> Option<u8> {
        let depth_inner = depth_elem.inner_html();
        let mut space_check = depth_inner.split_whitespace();
        let s = space_check.next().unwrap();

        let mut space_check = s.split("&");
        let depth = space_check.next().unwrap();

        match depth.parse::<u8>() {
            Ok(depth) => Some(depth),
            Err(_) => None,
        }
    }

    fn parse_current_draw(&self, elem: ElementRef) -> Option<u8> {
        let raw_val = elem.inner_html();
        let mut split_whitespace = raw_val.split_whitespace();

        let width = split_whitespace.next().unwrap();

        match width.parse::<u8>() {
            Ok(width) => Some(width),
            Err(_) => None,
        }
    }
}

impl Default for ModularGridScraper {
    fn default() -> ModularGridScraper {
        ModularGridScraper {
            module_selector: Selector::parse(".modules table tbody tr td:first-of-type a").unwrap(),
            module_name_selector: Selector::parse(".module-view-header h1").unwrap(),
            module_manufacturer_selector: Selector::parse(
                ".module-view-header .sub-header h2 a span",
            )
            .unwrap(),
            width_selector: Selector::parse(".box-specs div:first-of-type dl dd:first-of-type")
                .unwrap(),
            depth_selector: Selector::parse(".box-specs div:first-of-type dl dd:nth-of-type(2)")
                .unwrap(),
            positive_12_selector: Selector::parse(
                ".box-specs div:nth-of-type(2) dl dd:first-of-type",
            )
            .unwrap(),
            negative_12_selector: Selector::parse(
                ".box-specs div:nth-of-type(2) dl dd:nth-of-type(2)",
            )
            .unwrap(),
            positive_5_selector: Selector::parse(
                ".box-specs div:nth-of-type(2) dl dd:nth-of-type(3)",
            )
            .unwrap(),
            max_page: 2,
        }
    }
}

/// The state model
#[derive(Debug)]
enum ModularGridState {
    ModuleListPage(usize),
    Module,
}

/// The ouput the scraper should eventually produce
#[derive(Debug)]
struct Module {
    name: String,
    manufacturer: String,
    width: Option<u8>,
    depth: Option<u8>,
    current: CurrentDraw,
}

impl Scraper for ModularGridScraper {
    type Output = Module;
    type State = ModularGridState;

    /// do your scraping
    fn scrape(
        &mut self,
        response: Response<Self::State>,
        crawler: &mut Crawler<Self>,
    ) -> Result<Option<Self::Output>, anyhow::Error> {
        let html = response.html();

        if let Some(state) = response.state {
            match state {
                ModularGridState::ModuleListPage(page) => {
                    // find all entries
                    for link in html
                        .select(&self.module_selector)
                        .filter_map(|el| el.value().attr("href"))
                    {
                        // submit an url to a module
                        crawler.visit_with_state(
                            // &format!("https://www.modulargrid.net/e/modules/index/sort:Module.name/direction:asc/page:{}", page),
                            &format!("https://www.modulargrid.net{}", link),
                            ModularGridState::Module,
                        );
                    }
                    if page < self.max_page {
                        // queue in next page
                        crawler.visit_with_state(
                            &format!("https://www.modulargrid.net/e/modules/index/sort:Module.name/direction:asc/page:{}", page + 1),
                            ModularGridState::ModuleListPage(page + 1),
                        );
                    }
                }

                ModularGridState::Module => {
                    let name = html.select(&self.module_name_selector).next().unwrap();
                    let manufacturer = html
                        .select(&self.module_manufacturer_selector)
                        .next()
                        .unwrap();
                    let width = match html.select(&self.width_selector).next() {
                        Some(elem) => self.parse_width(elem),
                        None => None,
                    };

                    let depth = match html.select(&self.depth_selector).next() {
                        Some(elem) => self.parse_depth(elem),
                        None => None,
                    };

                    let positive_12 = match html.select(&self.positive_12_selector).next() {
                        Some(elem) => self.parse_current_draw(elem),
                        None => None,
                    };

                    let negative_12 = match html.select(&self.negative_12_selector).next() {
                        Some(elem) => self.parse_current_draw(elem),
                        None => None,
                    };

                    let positive_5 = match html.select(&self.positive_5_selector).next() {
                        Some(elem) => self.parse_current_draw(elem),
                        None => None,
                    };

                    // scrape the entry
                    let entry = Module {
                        name: name.inner_html(),
                        manufacturer: manufacturer.inner_html(),
                        width: width,
                        depth: depth,
                        current: CurrentDraw {
                            positive_12: positive_12,
                            negative_12: negative_12,
                            positive_5: positive_5,
                        },
                    };
                    return Ok(Some(entry));
                }
            }
        }

        Ok(None)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // only fulfill requests to `www.modulargrid.net`
    let config = CrawlerConfig::default().allow_domain_with_delay(
        "www.modulargrid.net",
        // add a delay between requests
        RequestDelay::Fixed(std::time::Duration::from_millis(2_000)),
    );

    let mut collector = Collector::new(ModularGridScraper::default(), config);

    collector.crawler_mut().visit_with_state(
        "https://www.modulargrid.net/e/modules/index/sort:Module.name/direction:asc/page:1",
        ModularGridState::ModuleListPage(1),
    );

    while let Some(output) = collector.next().await {
        let module = output?;
        dbg!(module);
    }

    Ok(())
}
