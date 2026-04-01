#!/bin/bash
# Naprawa organizacji repozytorium ZeroClaw
# Usage: ./scripts/fix-symlinks.sh [--dry-run]

set -e

DRY_RUN=""
if [ "$1" = "--dry-run" ]; then
    DRY_RUN="echo [DRY RUN]"
    echo "🔍 Mode: DRY RUN (nic nie zostanie zmienione)"
fi

echo "🔧 ZeroClaw Repository Fix Script"
echo ""

# 1. Sprawdź symlink ~/zero-backend
echo "=== 1. Sprawdzanie symlink ~/zero-backend ==="
if [ -L "$HOME/zero-backend" ]; then
    TARGET=$(readlink -f "$HOME/zero-backend")
    echo "Znaleziono symlink: ~/zero-backend -> $TARGET"
    echo ""
    echo "UWAGA: Ten symlink jest mylący!"
    echo "- Niby 'zero-backend' to Development workspace"
    echo "- Ale w rzeczywistości to tylko symlink do Research/zero-backend"
    echo ""
    read -p "Czy usunąć ten symlink? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        $DRY_RUN rm "$HOME/zero-backend"
        echo "✅ Usunięto symlink ~/zero-backend"
        echo "   Używaj bezpośrednio: ~/Research/zero-backend"
    else
        echo "⏭️  Pozostawiono symlink ~/zero-backend"
    fi
else
    echo "✅ Symlink ~/zero-backend nie istnieje (dobrze!)"
fi
echo ""

# 2. Sprawdź ~/.local/bin/zeroclaw-cli
echo "=== 2. Sprawdzanie ~/.local/bin/zeroclaw-cli ==="
if [ -L "$HOME/.local/bin/zeroclaw-cli" ]; then
    TARGET=$(readlink -f "$HOME/.local/bin/zeroclaw-cli")
    echo "Znaleziono symlink: ~/.local/bin/zeroclaw-cli -> $TARGET"
    echo ""
    if [[ "$TARGET" == *"Research/zero-backend/target/release"* ]]; then
        echo "❌ PROBLEM: Ten symlink wskazuje na DEVELOPMENT build!"
        echo "   Powinien wskazywać na PRODUCTION: ~/.cargo/bin/zeroclaw"
        echo ""
        read -p "Czy naprawić ten symlink? (y/N) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            $DRY_RUN rm "$HOME/.local/bin/zeroclaw-cli"
            $DRY_RUN ln -s "$HOME/.cargo/bin/zeroclaw" "$HOME/.local/bin/zeroclaw-cli"
            echo "✅ Naprawiono symlink ~/.local/bin/zeroclaw-cli -> ~/.cargo/bin/zeroclaw"
        else
            echo "⏭️  Pozostawiono symlink bez zmian"
        fi
    else
        echo "✅ Symlink wskazuje na: $TARGET"
    fi
else
    echo "ℹ️  Symlink ~/.local/bin/zeroclaw-cli nie istnieje"
fi
echo ""

# 3. Pokaż obecną strukturę binarek
echo "=== 3. Obecna struktura binarek ==="
echo "Production (z cargo install):"
ls -lh "$HOME/.cargo/bin/zeroclaw" 2>/dev/null || echo "  ❌ Nie znaleziono"
echo ""
echo "Development (z repo):"
ls -lh "$HOME/Research/zero-backend/target/release/zeroclaw" 2>/dev/null || echo "  ❌ Nie znaleziono"
echo ""
echo "PATH (which zeroclaw):"
which zeroclaw
echo ""

# 4. Zlicz skrypty wariantów
echo "=== 4. Skrypty wariantów w ~/.local/bin/ ==="
VARIANT_COUNT=$(ls -1 "$HOME/.local/bin/zeroclaw-*" 2>/dev/null | wc -l)
echo "Znaleziono $VARIANT_COUNT skryptów zeroclaw-*"
echo ""
if [ $VARIANT_COUNT -gt 10 ]; then
    echo "⚠️  Wiele wariantów - rozważ sprzątanie:"
    echo "   Zachowaj: zeroclaw-tui (wrapper do TUI)"
    echo "   Usuń: stare testy (zeroclaw-*, zeroclaw-enhanced, etc.)"
    echo ""
    read -p "Pokazać listę wszystkich skryptów? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        ls -1 "$HOME/.local/bin/zeroclaw-*" | head -20
        echo "... i więcej"
    fi
fi
echo ""

# 5. Weryfikacja PATH
echo "=== 5. Weryfikacja PATH ==="
echo "PATH order:"
echo "$PATH" | tr ':' '\n' | grep -E 'cargo|local/bin' | nl
echo ""
if echo "$PATH" | grep -q "$HOME/.cargo/bin:before.*$HOME/.local/bin"; then
    echo "✅ PATH poprawny: ~/.cargo/bin jest przed ~/.local/bin"
else
    echo "⚠️  Sprawdź czy ~/.cargo/bin jest przed ~/.local/bin w PATH"
fi
echo ""

# 6. Podsumowanie
echo "=== 6. Podsumowanie i rekomendacje ==="
echo ""
echo "✅ Jedno źródło prawdy:"
echo "   Dokumentacja: ~/Research/zero-backend/docs/INSTALLATION.md"
echo ""
echo "📋 Kluczowe lokalizacje:"
echo "   Development:  ~/Research/zero-backend/"
echo "   Production:  ~/.cargo/bin/zeroclaw"
echo "   Config:      ~/.zeroclaw/config.toml"
echo ""
echo "🔄 Standardowy workflow:"
echo "   1. Development: cd ~/Research/zero-backend && vim src/..."
echo "   2. Build:      cargo build --release"
echo "   3. Install:    cargo install --path ."
echo "   4. Verify:     ~/.cargo/bin/zeroclaw status"
echo ""
if [ -n "$DRY_RUN" ]; then
    echo "🔍 DRY RUN zakończony - uruchom bez --dry-run aby zastosować zmiany"
else
    echo "✅ Naprawa zakończona!"
fi
