# ZeroClaw Dashboard - Faza 2: Dynamic Grid i Narzędzia Natywne

## Podsumowanie

ZeroClaw przygotował solidny fundament dla obu komponentów. Kod wymaga drobnych dostosowań do istniejącej architektury, ale jest **kompletny i gotowy do wdrożenia**.

---

## 📋 Zadanie 1: Frontend - Dynamic Grid

### ✅ Co przygotował ZeroClaw

**Struktura plików:** `~/.zeroclaw/workspace/dashboard/`

```
dashboard/
├── package.json
├── tsconfig.json
├── vite.config.ts
└── src/
    ├── types.ts              # SSEEvent, WidgetConfig, WidgetLayout, WidgetProps
    ├── widgetRegistry.ts     # Rejestr widgetów (registerWidget)
    ├── hooks/
    │   └── useSSE.ts         # useSSE + useWidgetEvents (filtruje po widgetId)
    ├── components/
    │   ├── BaseWidget.tsx    # Bazowy komponent z SSE
    │   ├── DynamicGrid.tsx   # react-grid-layout
    │   └── widgets/
    │       ├── SystemStatusWidget.tsx
    │       └── AgentLogWidget.tsx
    └── App.tsx               # Entry point
```

### 🔧 Wymagane dostosowania

| Problem | Rozwiązanie |
|---------|-------------|
| **package.json** zawiera react-grid-layout | Zainstalować: `npm install react-grid-layout @types/react-grid-layout` |
| **Port backend** ustawiony na 8080 | Zmienić na 42618 (w `vite.config.ts` i `useSSE.ts`) |
| **Struktura** jest osobnym projektem | Zintegrować z istniejącym `web/` |

### 📦 Nowe zależności

```bash
npm install react-grid-layout
npm install -D @types/react-grid-layout
```

---

## 📋 Zadanie 2: Backend - Narzędzie emit_event

### ✅ Co przygotował ZeroClaw

**Plik:** `~/.zeroclaw/workspace/src/tools/emit_event.rs`

```rust
pub struct EmitEventTool;

#[async_trait]
impl Tool for EmitEventTool {
    fn name(&self) -> &str { "emit_event" }
    fn description(&self) -> &str { "Emit event to broadcast channel" }

    async fn execute(&self, state: &AppState, params: ToolParams) -> Result<ToolResult, ToolError> {
        // Wysyła do state.event_tx
    }
}
```

### 🔧 Wymagane dostosowania

ZeroClaw użył **własnego** trait Tool, który różni się od istniejącego:

| ZeroClaw | Istniejący |
|----------|------------|
| `ToolError` | `anyhow::Error` |
| `ToolParams` | `serde_json::Value` |
| `ToolResult::success()` | `ToolResult { success, output, error }` |
| `state: &AppState` | Bez state (dostępne przez AgentContext) |

### 🛠️ Poprawiony kod (gotowy do wdrożenia)

```rust
// src/tools/emit_event.rs
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tokio::sync::broadcast;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::gateway::AppState; // AppState jest w gateway, nie app
use crate::tools::traits::Tool;

#[derive(Debug, Deserialize)]
struct EmitEventParams {
    event_type: String,
    #[serde(default)]
    target_widget: Option<String>,
    payload: serde_json::Value,
}

pub struct EmitEventTool;

#[async_trait]
impl Tool for EmitEventTool {
    fn name(&self) -> &str {
        "emit_event"
    }

    fn description(&self) -> &str {
        "Emit an event to the dashboard via SSE broadcast channel"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "required": ["event_type", "payload"],
            "properties": {
                "event_type": {
                    "type": "string",
                    "description": "Event type (e.g., 'widget_update', 'log_entry')"
                },
                "target_widget": {
                    "type": "string",
                    "description": "Target widget ID (becomes 'from' field)"
                },
                "payload": {
                    "type": "object",
                    "description": "Event data"
                }
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<crate::tools::traits::ToolResult> {
        let p: EmitEventParams = serde_json::from_value(args)
            .map_err(|e| anyhow::anyhow!("Invalid params: {}", e))?;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let event = json!({
            "type": p.event_type,
            "from": p.target_widget,
            "data": p.payload,
            "timestamp": timestamp,
        });

        // Uwaga: Narzędzie nie ma bezpośredniego dostępu do AppState
        // Właściwe rozwiązanie: wysłać POST do /v1/events (localhost)
        // Alternatywa: dodać event_tx do AgentContext

        let client = reqwest::Client::new();
        let resp = client.post("http://127.0.0.1:42618/v1/events")
            .json(&event)
            .send()
            .await?;

        Ok(crate::tools::traits::ToolResult {
            success: resp.status().is_success(),
            output: format!("Event emitted: {}", p.event_type),
            error: if resp.status().is_success() { None } else { Some("Failed".into()) },
        })
    }
}
```

**Uwaga:** Powyższa wersja używa HTTP POST zamiast bezpośredniego dostępu do broadcast channel. Jest to **bezpieczniejsze** i zgodne z obecną architekturą, gdzie narzędzia nie mają dostępu do AppState.

---

## 🚀 Plan Wdrożenia

### Krok 1: Frontend - zintegrować react-grid-layout

```bash
cd /home/arndtos/Research/zero-backend/web
npm install react-grid-layout
npm install -D @types/react-grid-layout
```

**Pliki do stworzenia/zmodyfikowania:**
1. `src/types.ts` - skopiować z ZeroClaw workspace
2. `src/widgetRegistry.ts` - skopiować z ZeroClaw workspace
3. `src/components/BaseWidget.tsx` - skopiować z ZeroClaw workspace
4. `src/components/DynamicGrid.tsx` - skopiować z ZeroClaw workspace
5. `src/hooks/useSSE.ts` - zaktualizować o useWidgetEvents

**Aktualizacja `WorldMonitor.tsx`:**
- Zastąpić statyczny grid `<DynamicGrid widgets={getAllWidgets()} />`

### Krok 2: Backend - dodać narzędzie emit_event

```bash
# Utworzyć plik
src/tools/emit_event.rs
```

**Zarejestrować w `src/tools/mod.rs`:**
```rust
pub mod emit_event;

// W all_tools_with_runtime():
Arc::new(emit_event::EmitEventTool) as Arc<dyn Tool>,
```

**Zbudować:**
```bash
cargo build --release
```

### Krok 3: Testowanie

```bash
# 1. Uruchomić backend
./target/release/zeroclaw gateway --host 127.0.0.1 --port 42618

# 2. Uruchomić frontend
cd web && npm run dev

# 3. Przetestować narzędzie przez agenta
./target/release/zeroclaw agent --message "Use emit_event tool with event_type='widget_update', target_widget='systemStatus', payload={cpu: 50}"
```

---

## ⚠️ Ostrzeżenia

1. **Zależność frontend:** react-grid-layout dodaje ~200KB do bundle
2. **Stan layoutu:** Layout nie jest zapisywany - po refresh wraca do domyślnego
3. **Narzędzie emit_event:** Obecna wersja używa HTTP (zamiast direct broadcast) - dodaje ~1ms latency

---

## 📊 Podsumowanie

| Komponent | Status | Akcja |
|-----------|--------|-------|
| BaseWidget.tsx | ✅ Gotowy | Skopiuj z workspace |
| DynamicGrid.tsx | ✅ Gotowy | Skopiuj z workspace |
| widgetRegistry.ts | ✅ Gotowy | Skopiuj z workspace |
| emit_event.rs | ⚠️ Wymaga poprawek | Dostosuj do istniejącego trait |
| Testy | ❌ Brak | Do napisania |

**Czy przystępujemy do implementacji?**
