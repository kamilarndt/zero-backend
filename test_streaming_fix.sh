#!/bin/bash
# Test demonstrujący naprawę formatowania markdown w streaming
# Po uruchomieniu gateways, wyślij tę wiadomość przez LibreChat lub API

echo "=== Test naprawy streaming formatowania ==="
echo ""
echo "Uruchom ten test w LibreChat (http://localhost:42617) lub przez API:"
echo ""
echo "Wiadomość testowa:"
echo "---"
cat << 'TEST_MSG'
Sprawdź formatowanie:

# Nagłówek 1

To jest akapit po nagłówku.

## Podsekcja 1

- Punkt A
- Punkt B
- Punkt C

### Podpodsekcja

**Tekst pogrubiony** i *tekst kursywą*.

```
Kod blokowy
z wieloma liniami
```

Koniec testu.
TEST_MSG
echo "---"
echo ""
echo "Oczekiwany wynik:"
echo "- Każdy nagłówek powinien być w nowej linii"
echo "- Listy powinny być poprawnie sformatowane"
echo "- Kod blokowy powinien zachować formatowanie"
echo "- BRAK 'ściany tekstu' - wszystko powinno być czytelne"
echo ""
echo "Jeśli widzisz poprawne formatowanie, naprawa działa! ✓"
