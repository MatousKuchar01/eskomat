use crate::{Coffee, RATING_MAX};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Client;
use reqwest::Error;
use scraper::{Html, Selector};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

/// Doména e-shopu, doplňuje se před relativní odkazy (`/kmen/` -> `https://www.eskomat.cz/kmen/`)
const BASE_URL: &str = "https://www.eskomat.cz";

/// Kategorie se vší kávou. Obsahuje i kapsle a drip sety, ty se při parsování vynechají, protože nemají gramáž.
const CATEGORY_URL: &str = "https://www.eskomat.cz/kava/";

/// Cesta kategorie Pražírny. V hlavním menu webu má podkategorie jednotlivých pražíren.
const ROASTERS_PATH: &str = "/prazirny/";

/// Text pražírny u kávy, kterou se nepodařilo najít na stránce žádné pražírny.
const UNKNOWN_ROASTER: &str = "Neznámá pražírna";

/// Pražírna z hlavního menu webu (podkategorie `Pražírny`), např. `Kmen` -> `https://www.eskomat.cz/kmen/`
#[derive(Debug, PartialEq)]
pub struct Roaster {
    pub name: String,
    pub url: String,
}

pub fn fetch_coffees() -> Result<Vec<Coffee>, Error> {
    let client = Client::builder()
        .user_agent(concat!("eskomat-cli/", env!("CARGO_PKG_VERSION")))
        .build()?;

    let mut all_coffees = Vec::new();
    let mut roasters = Vec::new();
    let mut seen_urls = HashSet::new();
    let mut page = 1; // (/kava/, /kava/strana-2/, /kava/strana-3/...)

    let pb = ProgressBar::new_spinner();

    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("|/-\\ ")
            .template("{spinner:.bold.green} {msg}")
            .unwrap(),
    );

    pb.enable_steady_tick(Duration::from_millis(80));

    loop {
        pb.set_message(format!(
            "Stahuji Eskomat.cz | Načteno {} káv...",
            all_coffees.len()
        ));

        let html_content = client.get(page_url(CATEGORY_URL, page)).send()?.text()?;

        // seznam pražíren je v hlavním menu, které je na každé stránce; stačí ho vzít z první
        if page == 1 {
            roasters = parse_roasters(&html_content);
        }

        let coffees_on_page = parse_coffees(&html_content);

        let mut new_coffees_count = 0;

        for coffee in coffees_on_page {
            // stejná káva je na webu jen pod jednou URL, tou se odliší i stejná káva v jiném balení
            if seen_urls.insert(coffee.url.clone()) {
                all_coffees.push(coffee);
                new_coffees_count += 1;
            }
        }

        // Shoptet vrátí i stránku za poslední stránkou, jen bez produktů
        if new_coffees_count == 0 {
            break;
        }

        page += 1;

        // pro vývoj
        /*if page > 2 {
            break;
        }*/
    }

    fetch_roasters(&client, &mut all_coffees, &roasters, &pb);

    pb.finish_with_message(format!(
        " Staženo celkem {} káv z Eskomat.cz\n",
        all_coffees.len()
    ));

    Ok(all_coffees)
}

/// Stáhne stránku každé pražírny a podle URL produktů doplní kávám pražírnu a odkaz na ni.
/// Ve výpisu kategorie pražírna u kávy není, je jen v názvu kávy.
/// Káva, která není na stránce žádné pražírny (nebo se stránku nepodařilo stáhnout), zůstane s `Neznámá pražírna`.
fn fetch_roasters(client: &Client, coffees: &mut [Coffee], roasters: &[Roaster], pb: &ProgressBar) {
    let mut roaster_by_product_url: HashMap<String, &Roaster> = HashMap::new();

    for (index, roaster) in roasters.iter().enumerate() {
        pb.set_message(format!(
            "Stahuji pražírny {}/{}...",
            index + 1,
            roasters.len()
        ));

        for product_url in fetch_product_urls(client, &roaster.url) {
            roaster_by_product_url.entry(product_url).or_insert(roaster);
        }
    }

    for coffee in coffees.iter_mut() {
        if let Some(roaster) = roaster_by_product_url.get(&coffee.url) {
            coffee.roaster = roaster.name.clone();
            coffee.roaster_url = roaster.url.clone();
        }
    }
}

/// Stáhne všechny stránky kategorie (např. pražírny) a vrátí URL produktů v ní.
/// Když se některá stránka nepodaří stáhnout, vrátí to, co se stihlo načíst.
fn fetch_product_urls(client: &Client, category_url: &str) -> Vec<String> {
    let mut product_urls = Vec::new();
    let mut seen_urls = HashSet::new();
    let mut page = 1;

    loop {
        let Ok(html_content) = client
            .get(page_url(category_url, page))
            .send()
            .and_then(|response| response.text())
        else {
            break;
        };

        let mut new_urls_count = 0;

        for product_url in parse_product_urls(&html_content) {
            if seen_urls.insert(product_url.clone()) {
                product_urls.push(product_url);
                new_urls_count += 1;
            }
        }

        if new_urls_count == 0 {
            break;
        }

        page += 1;
    }

    product_urls
}

/// URL stránky kategorie: první stránka je kategorie samotná, další mají tvar `<kategorie>strana-2/`
fn page_url(category_url: &str, page: u32) -> String {
    if page <= 1 {
        category_url.to_string()
    } else {
        format!("{category_url}strana-{page}/")
    }
}

/// Vytáhne seznam pražíren z hlavního menu webu.
///
/// Menu má položku `Pražírny` (`li.has-third-level` s odkazem na `/prazirny/`) a pod ní
/// `ul.menu-level-3` s odkazy na kategorie jednotlivých pražíren (`/kmen/`, `/dos-mundos/`...).
/// Menu je na stránce vícekrát (desktop i mobil), pražírny se proto odlišují podle URL.
fn parse_roasters(html: &str) -> Vec<Roaster> {
    let document = Html::parse_document(html);

    let menu_item_selector = Selector::parse("li.has-third-level").unwrap();
    let roasters_link_selector = Selector::parse(&format!("a[href$=\"{ROASTERS_PATH}\"]")).unwrap();
    let submenu_link_selector = Selector::parse("ul.menu-level-3 a[href]").unwrap();

    let mut roasters = Vec::new();
    let mut seen_urls = HashSet::new();

    for menu_item in document.select(&menu_item_selector) {
        if menu_item.select(&roasters_link_selector).next().is_none() {
            continue;
        }

        for link in menu_item.select(&submenu_link_selector) {
            let name = normalize_whitespace(&link.text().collect::<String>());
            let url = absolute_url(link.value().attr("href").unwrap_or(""));

            if name.is_empty() || url.is_empty() || !seen_urls.insert(url.clone()) {
                continue;
            }

            roasters.push(Roaster { name, url });
        }
    }

    roasters
}

/// Vytáhne URL produktů z výpisu kategorie (`div#products a.name`)
fn parse_product_urls(html: &str) -> Vec<String> {
    let document = Html::parse_document(html);
    let link_selector = Selector::parse("div#products a.name[href]").unwrap();

    document
        .select(&link_selector)
        .map(|link| absolute_url(link.value().attr("href").unwrap_or("")))
        .filter(|url| !url.is_empty())
        .collect()
}

/// Parsuje HTML dokument a extrahuje z něj seznam produktů kávy.
///
/// Funkce prochází HTML strukturu pomocí CSS selektorů a vyhledává elementy reprezentující
/// jednotlivé produkty ve výpisu kategorie (`div#products div.p[data-micro="product"]`).
/// U každé položky se pokouší extrahovat název, odkaz na detail, hodnocení, hmotnost, cenu,
/// původní cenu, dostupnost a chuťový profil. Pražírna se doplňuje až později ze stránek pražíren.
///
/// # Chování a fallbacky
///
/// * **Přeskakování:** Pokud položka nemá název, má název prázdný, její výsledná cena je `0.0`
///   nebo se nepodaří zjistit hmotnost (kapsle, drip sety), do výsledného seznamu se **nepřidá**.
/// * **Extrakce hmotnosti:** Hmotnost se primárně hledá v názvu kávy (`250g`, `1kg`). Pokud tam není,
///   dopočítá se z měrné ceny za 100 g (`div.price-measure`).
/// * **Cena:** Primárně z atributu `data-micro-price` nabídky, jinak z textu `div.price-final`.
/// * **Chuťový profil:** První řádek krátkého popisu (`p.p-desc`), kde web uvádí chuťové tóny oddělené čárkou.
///
/// # Arguments
///
/// * `html` - Vstupní textový řetězec obsahující HTML kód stránky (např. stažený e-shop).
///
/// # Returns
///
/// Vrací `Vec<Coffee>` obsahující všechny úspěšně na parsované kávy. Pokud HTML neobsahuje žádné odpovídající prvky, vrátí prázdný vektor.
fn parse_coffees(html: &str) -> Vec<Coffee> {
    let document = Html::parse_document(html);

    // Selektory z přiloženého HTML (Shoptet šablona)
    let item_selector = Selector::parse("div#products div.p[data-micro=\"product\"]").unwrap();
    let name_selector = Selector::parse("a.name").unwrap();
    let rating_selector = Selector::parse("div.stars-wrapper[data-micro-rating-value]").unwrap();
    let offer_selector = Selector::parse("[data-micro=\"offer\"]").unwrap();
    let price_selector = Selector::parse("div.price-final").unwrap();
    let old_price_selector = Selector::parse("span.price-standard").unwrap();
    let unit_price_selector = Selector::parse("div.price-measure").unwrap();
    let stock_selector = Selector::parse("div.availability").unwrap();
    let flavors_selector = Selector::parse("p.p-desc").unwrap();

    let mut coffees = Vec::new();

    for item in document.select(&item_selector) {
        // extrakce názvu kávy a odkazu na detail
        let (name, url) = match item.select(&name_selector).next() {
            Some(el) => (
                normalize_whitespace(&el.text().collect::<String>()),
                absolute_url(el.value().attr("href").unwrap_or("")),
            ),
            None => continue, // Pokud káva nemá název, přeskočíme
        };

        if name.is_empty() {
            continue;
        }

        // extrakce hodnocení kávy (průměr hvězd), káva bez recenzí má místo hvězd jen placeholder
        let rating = item
            .select(&rating_selector)
            .next()
            .and_then(|el| el.value().attr("data-micro-rating-value"))
            .and_then(extract_rating);

        // extrakce celkové ceny
        let mut price_czk: f64 = item
            .select(&offer_selector)
            .next()
            .and_then(|el| el.value().attr("data-micro-price"))
            .and_then(|price| price.trim().parse::<f64>().ok())
            .unwrap_or(0.0);

        if price_czk <= 0.0
            && let Some(price_el) = item.select(&price_selector).next()
        {
            price_czk = extract_price(&price_el.text().collect::<String>());
        }

        // extrakce původní ceny u zlevněné kávy
        let old_price_czk = item
            .select(&old_price_selector)
            .next()
            .map(|el| extract_price(&el.text().collect::<String>()))
            .filter(|old| *old > price_czk);

        // extrakce gramáže
        let mut weight_g = extract_weight(&name);

        if weight_g == 0
            && let Some(unit_price_el) = item.select(&unit_price_selector).next()
        {
            weight_g = weight_from_unit_price(price_czk, &unit_price_el.text().collect::<String>());
        }

        // extrakce dostupnosti
        let stock = match item.select(&stock_selector).next() {
            Some(el) => normalize_whitespace(&el.text().collect::<String>()),
            None => "Neuvedeno".to_string(),
        };

        // extrakce chuti
        let flavors = match item.select(&flavors_selector).next() {
            Some(el) => extract_flavors(&el.text().collect::<String>()),
            None => String::new(),
        };

        // kapsle a drip sety nemají gramáž, cena za 100 g u nich nedává smysl
        if price_czk > 0.0 && weight_g > 0 {
            coffees.push(Coffee {
                name,
                rating,
                roaster: UNKNOWN_ROASTER.to_string(),
                roaster_url: String::new(),
                weight_g,
                price_czk,
                old_price_czk,
                stock,
                flavors,
                url,
            });
        }
    }

    coffees
}

/// Doplní relativní cestu o doménu webu, absolutní URL nechá být, prázdný odkaz vrátí prázdný.
fn absolute_url(href: &str) -> String {
    let href = href.trim();

    if href.starts_with("http") {
        href.to_string()
    } else if !href.is_empty() {
        format!("{BASE_URL}{href}")
    } else {
        String::new()
    }
}

/// Sloučí všechny bílé znaky (včetně nových řádků a pevných mezer) do jednoduchých mezer a ořízne okraje.
fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<&str>>().join(" ")
}

/// Vrátí chuťový profil z krátkého popisu kávy. Web uvádí na prvním řádku popisu chuťové tóny
/// oddělené čárkou (např. `Broskev, Květiny, Mléčná čokoláda`), další řádky jsou běžný popis.
fn extract_flavors(description: &str) -> String {
    description
        .lines()
        .map(normalize_whitespace)
        .find(|line| !line.is_empty())
        .unwrap_or_default()
}

/// Extrahuje průměrné hodnocení (počet hvězd) z textu, např. `"4,5"` nebo `"3.9"`.
fn extract_rating(text: &str) -> Option<f64> {
    text.trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|rating| *rating > 0.0 && *rating <= RATING_MAX)
}

/// Extrahuje hmotnost v gramech z textového řetězce.
///
/// Funkce v textu vyhledá číselnou hodnotu s jednotkou gramů nebo kilogramů, přičemž jednotka
/// může být přilepená k číslu (`250g`, `1kg`) nebo oddělená mezerou (`250 g`, `0,5 kg`).
/// Aby byla hmotnost úspěšně extrahována, musí splňovat rozsah **od 50 g do 5000 g** (včetně).
///
/// Pokud text neobsahuje žádné platné číslo s jednotkou hmotnosti (např. `10 ks` u kapslí),
/// nebo pokud nalezená hmotnost leží mimo povolený rozsah, funkce **nevyvolá panic** a vrátí `0`.
///
/// # Arguments
///
/// * `text` - Vstupní textový řetězec (např. "Kmen Kafe kafe espresso 250g" nebo "Coming Out Filtr 1kg").
fn extract_weight(text: &str) -> u32 {
    let cleaned: String = text
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_digit() || c.is_alphabetic() || c == '.' {
                c
            } else if c == ',' {
                '.'
            } else {
                ' '
            }
        })
        .collect();

    let mut pending_number: Option<f64> = None;

    for word in cleaned.split_whitespace() {
        let (number, unit_multiplier) = if let Some(num_str) = word.strip_suffix("kg") {
            (num_str.parse::<f64>().ok().or(pending_number.filter(|_| num_str.is_empty())), 1000.0)
        } else if let Some(num_str) = word.strip_suffix('g') {
            (num_str.parse::<f64>().ok().or(pending_number.filter(|_| num_str.is_empty())), 1.0)
        } else {
            pending_number = word.parse::<f64>().ok();
            continue;
        };

        pending_number = None;

        if let Some(number) = number {
            let grams = (number * unit_multiplier).round() as u32;

            if (50..=5000).contains(&grams) {
                return grams;
            }
        }
    }

    0
}

/// Dopočítá hmotnost balení z ceny a měrné ceny za 100 g, např. cena `299,90 Kč`
/// a měrná cena `149,95 Kč / 100 g` -> `200` g.
///
/// Vrací `0`, když měrná cena není za 100 g (kapsle mají cenu za kus) nebo se ji nepodaří přečíst.
fn weight_from_unit_price(price_czk: f64, unit_price_text: &str) -> u32 {
    let Some((amount, unit)) = unit_price_text.split_once('/') else {
        return 0;
    };

    let unit: String = unit.split_whitespace().collect::<String>().to_lowercase();

    if unit != "100g" {
        return 0;
    }

    let price_per_100g = extract_price(amount);

    if price_per_100g <= 0.0 || price_czk <= 0.0 {
        return 0;
    }

    (price_czk / price_per_100g * 100.0).round() as u32
}

/// Extrahuje číselnou hodnotu ceny z textového řetězce.
///
/// Funkce vezme text před prvním výskytem zkratky "Kč" (pokud tam je),
/// odstraní z něj všechny znaky kromě číslic, teček a čárek,
/// převede čárky na tečky a pokusí se výsledek převést na `f64`.
///
/// Pokud se v textu žádné číslo nenajde nebo parsování selže,
/// funkce **nevyvolá panic**, ale vrátí hodnotu `0.0`.
///
/// # Arguments
///
/// * `text` - Vstupní textový řetězec, který obsahuje cenu (např. "Cena: 1 250,50 Kč").
fn extract_price(text: &str) -> f64 {
    let text_before_kc = text.split("Kč").next().unwrap_or(text);

    let cleaned: String = text_before_kc
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == ',' || *c == '.')
        .collect();

    let cleaned = cleaned.replace(',', ".");
    cleaned.parse::<f64>().unwrap_or(0.0)
}

#[cfg(test)]
mod tests;
