# eskomat -> scraper kávy z eskomat.cz

CLI nástroj, který stáhne nabídku zrnkové kávy z [eskomat.cz](https://www.eskomat.cz)

## Instalace

### Linux a macOS (doporučeno)

```sh
curl -fsSL https://github.com/MatousKuchar01/eskomat/releases/latest/download/eskomat-cli-installer.sh | sh
```

Skript stáhne hotovou binárku pro tvůj systém do `~/.cargo/bin` (nebo `~/.local/bin`) a přidá ji do `PATH`.
Po instalaci otevři nový terminál a napiš `eskomat`.

### Windows (PowerShell)

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/MatousKuchar01/eskomat/releases/latest/download/eskomat-cli-installer.ps1 | iex"
```

### Ručně

Na stránce [Releases](https://github.com/MatousKuchar01/eskomat/releases/latest) stáhni archiv pro svůj systém,
rozbal ho a binárku `eskomat` zkopíruj kamkoliv do `PATH`.

### Z Rust zdrojáků

Pokud máš nainstalovaný Rust (přes [rustup](https://rustup.rs)):

```sh
cargo install --git https://github.com/MatousKuchar01/eskomat
```

## Použití

```sh
eskomat                                # celá nabídka seřazená podle ceny za 100 g
eskomat --max-price 150                # jen kávy do 150 Kč / 100 g
eskomat --max-weight 250               # jen balení do 250 g
eskomat --roaster kmen                 # jen kávy od konkrétní pražírny
eskomat --search etiopie               # hledá v názvu kávy i v chuťovém profilu
eskomat -m 160 -w 500 -r kmen -s filtr # filtry jde kombinovat, krátké přepínače fungují taky
eskomat --on-sale                      # jen zlevněné kávy, sleva je v tabulce červeně u ceny
eskomat --refresh                      # ignoruje 12hodinovou cache a stáhne čerstvá data
eskomat --no-pager                     # vypíše tabulku přímo, bez procházení v less
eskomat --help
```

Tabulka obsahuje jen kávu prodávanou na váhu. Kapsle a drip sety z kategorie Káva se vynechávají,
protože u nich nejde spočítat cena za 100 g.

## Jak to funguje

1. Stáhne výpis kategorie [Káva](https://www.eskomat.cz/kava/) včetně dalších stránek (`/kava/strana-2/`...).
   Z výpisu bere název, cenu, původní cenu u slev, hodnocení, dostupnost a chuťový profil (první řádek krátkého popisu).
2. Pražírna u kávy ve výpisu není, proto stáhne stránku každé pražírny z menu `Pražírny` a kávy k nim přiřadí podle URL.
3. Výsledek uloží do cache (`~/.cache/eskomat/coffees.json`), která platí 12 hodin.
