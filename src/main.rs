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
    max_page: usize,
}

impl ModularGridScraper {
    fn parse_width(&self, width: ElementRef) -> Option<u8> {
        let width_inner = width.inner_html();
        let mut split_width = width_inner.split_whitespace();
        let width = split_width.next().unwrap();

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
            width_selector: Selector::parse(".box-specs dl dd:first-of-type").unwrap(),
            depth_selector: Selector::parse(".box-specs dl dd:nth-of-type(2)").unwrap(),
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
    depth: String,
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
                    let width = html.select(&self.width_selector).next().unwrap();
                    let depth = html.select(&self.depth_selector).next().unwrap();

                    // scrape the entry
                    let entry = Module {
                        name: name.inner_html(),
                        manufacturer: manufacturer.inner_html(),
                        width: self.parse_width(width),
                        depth: depth.inner_html(),
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
    // env_logger::init();

    // only fulfill requests to `news.ycombinator.com`
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
        let post = output?;
        dbg!(post);
    }

    Ok(())
}
