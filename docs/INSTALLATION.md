# ZeroClaw Installation Guide

**Single Source of Truth dla lokalizacji binarek i konfiguracji**

## 🎯 Szybka odpowiedź

| Pytanie | Odpowiedź |
|---------|-----------|
| Gdzie jest produkcyjny zeroclaw? | `~/.cargo/bin/zeroclaw` |
| Gdzie jest development wersja? | `~/Research/zero-backend/target/release/zeroclaw` |
| Gdzie jest konfiguracja? | `~/.zeroclaw/config.toml` |
| Jak zainstalować na produkcji? | `cd ~/Research/zero-backend && cargo install --path .` |
| Jak zaktualizować produkcję? | `cd ~/Research/zero-backend && git pull && cargo install --path .` |

## 📂 Struktura katalogów

```
/home/arndtos/
├── Research/zero-backend/     # ✅ DEVELOPMENT (git repo)
│   ├── .git/                  #    github.com/kamilarndt/zeroclaw-backend
│   ├── src/                   #    Kod źródłowy Rust
│   └── target/release/        #    Development builds
│
├── .cargo/                    # ✅ PRODUKCJA (cargo install)
│   └── bin/zeroclaw           #    Główna binarka (17MB)
│
├── .zeroclaw/                 # ✅ KONFIGURACJA
│   ├── config.toml            #    Główny plik konfiguracyjny
│   ├── storage/               #    Bazy danych (SQLite, Qdrant)
│   └── skills/                #    Skilli ZeroClaw
│
├── .local/bin/                # ⚠️  SKRYPTY HELPERS
│   ├── zeroclaw-cli -> ~/Research/zero-backend/target/release/zeroclaw  # ❌ DEPRECATED
│   └── zeroclaw-tui -> ~/.cargo/bin/zeroclaw-tui  # ✅ OK (wrapper do TUI)
│
└── zero-backend -> Research/zero-backend  # ⚠️  SYMLINK (mylący, można usunąć)
```

## 🔧 Development

### Praca nad kodem

```bash
# Zawsze pracuj w Research workspace
cd ~/Research/zero-backend

# Sprawdź branch
git branch

# Zbuduj development version
cargo build --release

# Testuj development build
./target/release/zeroclaw agent --message "test"
./target/release/zeroclaw-tui
```

**Binarka development:** `~/Research/zero-backend/target/release/zeroclaw`

## 🚀 Production Installation

### Metoda 1: Cargo Install (ZALECANE)

```bash
cd ~/Research/zero-backend
cargo install --path .
```

**Aktualizacja produkcji:**
```bash
cd ~/Research/zero-backend
git pull
cargo install --path .
```

## 🔍 Troubleshooting

### "Który zeroclaw jest uruchomiony?"

```bash
# Sprawdź PATH
which zeroclaw

# Sprawdź czy to symlink
ls -l $(which zeroclaw)

# Sprawdź wersję/wielkość
ls -lh $(which zeroclaw)
```

### "Chcę użyć development wersji"

```bash
# Użyj pełnej ścieżki
~/Research/zero-backend/target/release/zeroclaw agent --message "test"
```

## 📝 Podsumowanie

### Jedno źródło prawdy

**Pytanie:** "Gdzie jest zeroclaw?"

**Odpowiedź:** Sprawdź `which zeroclaw` - to pokazuje produkcyjną binarkę.

**Development:** Zawsze używaj pełnej ścieżki `~/Research/zero-backend/target/release/zeroclaw`

### Kluczowe lokalizacje

| Cel | Ścieżka |
|-----|---------|
| Development workspace | `~/Research/zero-backend/` |
| Git repository | `~/Research/zero-backend/.git/` |
| Production binary | `~/.cargo/bin/zeroclaw` |
| Configuration | `~/.zeroclaw/config.toml` |

---

**WERSJA:** 1.0  
**OSTATNIA AKTUALIZACJA:** 2026-04-01  
**STATUS:** ✅ Single Source of Truth ustalony
