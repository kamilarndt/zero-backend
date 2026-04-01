# ZeroClaw World Monitor

Prosty, generyczny dashboard do monitorowania systemu ZeroClaw w czasie rzeczywistym.

## Architektura

```
┌─────────────────────────────────────────────────────────────┐
│                     ZeroClaw Backend (Rust)                  │
│  ┌──────────────┐         ┌──────────────────────────┐     │
│  │  POST /v1/   │────────▶│  Event Broadcast Channel │     │
│  │   events     │         │  (tokio::sync::broadcast)│     │
│  └──────────────┘         └──────────────────────────┘     │
│                                     │                       │
│                                     ▼                       │
│  ┌──────────────┐   ┌────────────────────────────┐         │
│  │  GET /v1/    │◀──│   SSE Stream               │         │
│  events/stream  │   │   (real-time JSON events)   │         │
│  └──────────────┘   └────────────────────────────┘         │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                   Frontend (React + Vite)                   │
│  ┌────────────┐  ┌────────────┐  ┌──────────────────┐     │
│  │   System   │  │   Agent    │  │  (future widgets)│     │
│  │   Status   │  │    Log     │  │                  │     │
│  └────────────┘  └────────────┘  └──────────────────┘     │
└─────────────────────────────────────────────────────────────┘
```

## Endpointy API

### `POST /v1/events`
Wysyła event do systemu - event jest rozgłaszany do wszystkich połączonych klientów SSE.

**Payload:**
```json
{
  "type": "log_entry",        // typ eventu
  "from": "agent_name",       // źródło (opcjonalne)
  "to": "target_name",        // cel (opcjonalne)
  "data": {                   // dowolne dane
    "message": "Agent started task",
    "level": "info"
  }
}
```

**Typy eventów:**
- `log_entry` - wpis do logu aktywności
- `widget_update` - aktualizacja widgetu (np. systemStatus)
- `agent_action` - akcja agenta (np. A2A komunikacja)
- dowolny inny `type` - system jest generyczny

### `GET /v1/events/stream`
SSE stream z eventami w czasie rzeczywistym.

**Odpowiedź:**
```
data: {"type":"log_entry","from":"agent","data":{"message":"..."},"timestamp":...}
```

## Testowanie

### 1. Uruchom backend
```bash
cd /home/arndtos/Research/zero-backend
./target/release/zeroclaw gateway --host 127.0.0.1 --port 42617
```

### 2. Uruchom frontend
```bash
cd web
npm run dev
```
Otwórz: http://localhost:3000

### 3. Wyślij testowy event (curl)
```bash
# Log entry
curl -X POST http://localhost:42617/v1/events \
  -H "Content-Type: application/json" \
  -d '{
    "type": "log_entry",
    "from": "test_agent",
    "data": {
      "message": "Hello from curl!",
      "level": "info"
    }
  }'

# Agent action
curl -X POST http://localhost:42617/v1/events \
  -H "Content-Type: application/json" \
  -d '{
    "type": "agent_action",
    "from": "concierge",
    "to": "researcher",
    "data": {
      "action": "delegate_task"
    }
  }'

# System status update
curl -X POST http://localhost:42617/v1/events \
  -H "Content-Type: application/json" \
  -d '{
    "type": "widget_update",
    "from": "systemStatus",
    "data": {
      "cpu": 42,
      "memory": 58,
      "agents": 5,
      "uptime": "3h 27m"
    }
  }'
```

## Struktura Frontendu

```
web/
├── src/
│   ├── pages/
│   │   └── WorldMonitor.tsx      # Główna strona
│   ├── components/
│   │   ├── Widget.tsx            # Bazowy komponent widgetu
│   │   ├── SystemStatusWidget.tsx # Status systemu
│   │   └── AgentLogWidget.tsx     # Log aktywności
│   └── hooks/
│       └── useSSE.ts              # Universal SSE hook
```

## Rozszerzanie Systemu

Nowe widgety są dodawane przez:
1. Utworzenie komponentu React (np. `MyWidget.tsx`)
2. Dodanie go do `WorldMonitor.tsx`
3. Odbieranie odpowiednich eventów typu w `handleEvent()`

System jest **całkowicie generyczny** - żadne logika specyficzna dla konkretnych agentów nie jest wymagana.
