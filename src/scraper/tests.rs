use super::*;

const ITEM_REGULAR: &str = include_str!("fixtures/item_regular.html");
const ITEM_DISCOUNTED: &str = include_str!("fixtures/item_discounted.html");
const ITEM_RATED: &str = include_str!("fixtures/item_rated.html");
const ROASTER_MENU: &str = include_str!("fixtures/roaster_menu.html");
const ROASTER_PAGE: &str = include_str!("fixtures/roaster_page.html");

#[test]
fn parses_regular_coffee() {
    let coffees = parse_coffees(ITEM_REGULAR);

    assert_eq!(coffees.len(), 1);
    let coffee = &coffees[0];
    assert_eq!(coffee.name, "Kmen & Eskomat Coming Out filtr 250g");
    assert_eq!(coffee.roaster, "Neznámá pražírna", "pražírna se doplňuje až ze stránek pražíren");
    assert_eq!(coffee.roaster_url, "");
    assert_eq!(coffee.rating, None, "káva bez recenzí nemá hodnocení");
    assert_eq!(coffee.weight_g, 250);
    assert_eq!(coffee.price_czk, 399.9);
    assert_eq!(coffee.old_price_czk, None);
    assert_eq!(coffee.stock, "Skladem (>5 ks)");
    assert_eq!(coffee.flavors, "Broskev, Květiny, Mléčná čokoláda");
    assert_eq!(
        coffee.url,
        "https://www.eskomat.cz/kmen-eskomat-coming-out-filtr-250g/"
    );
}

#[test]
fn parses_discounted_coffee() {
    let coffees = parse_coffees(ITEM_DISCOUNTED);

    assert_eq!(coffees.len(), 1);
    let coffee = &coffees[0];
    assert_eq!(coffee.name, "The Naughty Dog HALLOWEEN filtr/espresso 200g");
    assert_eq!(coffee.price_czk, 299.9);
    assert_eq!(coffee.old_price_czk, Some(449.9));
    assert_eq!(coffee.weight_g, 200);
    assert_eq!(coffee.stock, "Skladem (2 ks)");
    assert_eq!(coffee.flavors, "Ovocný punč, Borůvky, Čokoládový cupcake");

    let percent = coffee.discount_percent().unwrap();
    assert!((percent - 33.3).abs() < 0.1, "sleva má být cca 33 %, je {percent}");
}

#[test]
fn parses_rated_coffee() {
    let coffees = parse_coffees(ITEM_RATED);

    assert_eq!(coffees.len(), 1);
    let coffee = &coffees[0];
    assert_eq!(coffee.name, "Father´s Coffee Roastery & Eskomat Superautomat espresso 250g");
    assert_eq!(coffee.rating, Some(5.0));
    assert_eq!(coffee.price_czk, 349.9);
    assert_eq!(coffee.flavors, "Mléčná čokoláda, Lískooříškové máslo, Višně, Švestky");
}

#[test]
fn takes_weight_from_unit_price_when_name_has_none() {
    let html = r#"
        <div id="products">
            <div class="p" data-micro="product">
                <a href="/etiopie-test/" class="name">Etiopie TEST</a>
                <div data-micro="offer" data-micro-price="299.90">
                    <div class="price price-final"><strong>299,90 Kč</strong></div>
                    <div class="price-measure"><span class="sr-only">Měrná cena: </span>149,95 Kč /&nbsp;100&nbsp;g</div>
                </div>
            </div>
        </div>
    "#;

    let coffees = parse_coffees(html);

    assert_eq!(coffees.len(), 1);
    assert_eq!(coffees[0].weight_g, 200);
    assert_eq!(coffees[0].url, "https://www.eskomat.cz/etiopie-test/");
    assert_eq!(coffees[0].stock, "Neuvedeno");
    assert_eq!(coffees[0].flavors, "");
}

#[test]
fn takes_price_from_text_when_offer_has_no_attribute() {
    let html = r#"
        <div id="products">
            <div class="p" data-micro="product">
                <a href="/etiopie-test-250g/" class="name">Etiopie TEST 250g</a>
                <div class="price price-final"><strong>1 299,90 Kč</strong></div>
            </div>
        </div>
    "#;

    let coffees = parse_coffees(html);

    assert_eq!(coffees.len(), 1);
    assert_eq!(coffees[0].price_czk, 1299.9);
}

#[test]
fn skips_capsules_without_weight() {
    let html = r#"
        <div id="products">
            <div class="p" data-micro="product">
                <a href="/kapsle-test/" class="name">Etiopie kávové kapsle 10 ks</a>
                <div data-micro="offer" data-micro-price="219.90">
                    <div class="price price-final"><strong>219,90 Kč</strong></div>
                    <div class="price-measure">21,99 Kč /&nbsp;1&nbsp;ks</div>
                </div>
            </div>
        </div>
    "#;

    assert!(parse_coffees(html).is_empty());
}

#[test]
fn skips_item_without_price() {
    let html = r#"
        <div id="products">
            <div class="p" data-micro="product">
                <a href="/etiopie-test-250g/" class="name">Etiopie TEST 250g</a>
            </div>
        </div>
    "#;

    assert!(parse_coffees(html).is_empty());
}

#[test]
fn ignores_products_outside_main_listing() {
    let html = r#"
        <div class="products-top-wrapper">
            <div class="p" data-micro="product">
                <a href="/etiopie-test-250g/" class="name">Etiopie TEST 250g</a>
                <div data-micro="offer" data-micro-price="300.00"></div>
            </div>
        </div>
    "#;

    assert!(parse_coffees(html).is_empty(), "karusel nejprodávanějších káv není součástí výpisu");
}

#[test]
fn parses_roasters_from_menu() {
    let roasters = parse_roasters(ROASTER_MENU);

    assert_eq!(roasters.len(), 12);
    assert_eq!(
        roasters[0],
        Roaster {
            name: "Kmen".to_string(),
            url: "https://www.eskomat.cz/kmen/".to_string(),
        }
    );
    assert_eq!(roasters[6].name, "Beansmith's", "HTML entity v názvu se dekóduje");
    assert_eq!(roasters[11].name, "The Naughty Dog");
}

#[test]
fn roasters_are_empty_when_menu_has_no_roasters_item() {
    let html = r#"
        <li class="has-third-level">
            <a href="/vybaveni/">Vybavení</a>
            <ul class="menu-level-3"><li><a href="/mlynky/">Mlýnky</a></li></ul>
        </li>
    "#;

    assert!(parse_roasters(html).is_empty());
}

#[test]
fn parses_product_urls_from_roaster_page() {
    assert_eq!(
        parse_product_urls(ROASTER_PAGE),
        vec![
            "https://www.eskomat.cz/kmen-eskomat-coming-out-filtr-250g/",
            "https://www.eskomat.cz/kmen-kafe-kafe-espresso-250g/",
        ]
    );
}

#[test]
fn builds_page_urls() {
    assert_eq!(page_url("https://www.eskomat.cz/kava/", 1), "https://www.eskomat.cz/kava/");
    assert_eq!(page_url("https://www.eskomat.cz/kava/", 2), "https://www.eskomat.cz/kava/strana-2/");
}

#[test]
fn extracts_weight_in_grams_and_kilograms() {
    assert_eq!(extract_weight("Kmen Kafe kafe espresso 250g"), 250);
    assert_eq!(extract_weight("Kmen & Eskomat Coming Out Filtr 1kg"), 1000);
    assert_eq!(extract_weight("Dos Mundos Dva světy espresso blend 500 g"), 500);
    assert_eq!(extract_weight("Zen filtr/espresso bez kofeinu 0,5 kg"), 500);
    assert_eq!(extract_weight("Etiopie kávové kapsle 10 ks"), 0);
    assert_eq!(extract_weight("Hario V60-02 filtry 40 ks"), 0);
    assert_eq!(extract_weight("Vzorek 20g"), 0, "mimo rozsah 50–5000 g");
}

#[test]
fn computes_weight_from_unit_price_only_per_100g() {
    assert_eq!(weight_from_unit_price(299.9, "Měrná cena: 149,95 Kč / 100 g"), 200);
    assert_eq!(weight_from_unit_price(1299.9, "129,99 Kč /\u{a0}100\u{a0}g"), 1000);
    assert_eq!(weight_from_unit_price(219.9, "21,99 Kč / 1 ks"), 0);
    assert_eq!(weight_from_unit_price(299.9, "bez lomítka"), 0);
    assert_eq!(weight_from_unit_price(0.0, "149,95 Kč / 100 g"), 0);
}

#[test]
fn extracts_flavors_from_first_line_of_description() {
    assert_eq!(
        extract_flavors("\n    Broskev,  Květiny, Mléčná čokoláda\nNaše úplně první káva...\n"),
        "Broskev, Květiny, Mléčná čokoláda"
    );
    assert_eq!(extract_flavors("   \n  "), "");
}

#[test]
fn extracts_rating_with_comma_or_dot_and_rejects_out_of_range() {
    assert_eq!(extract_rating("4,5"), Some(4.5));
    assert_eq!(extract_rating(" 3.9 "), Some(3.9));
    assert_eq!(extract_rating("5"), Some(5.0));
    assert_eq!(extract_rating("0"), None);
    assert_eq!(extract_rating("5,1"), None);
    assert_eq!(extract_rating("abc"), None);
}

#[test]
fn extracts_price_with_thousands_separator() {
    assert_eq!(extract_price("1 299,90 Kč"), 1299.9);
    assert_eq!(extract_price("Původně: 449,90 Kč"), 449.9);
    assert_eq!(extract_price("\u{a0}"), 0.0);
}
