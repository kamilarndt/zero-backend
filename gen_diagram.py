#!/usr/bin/env python3
"""Generate ZeroClaw Dashboard Architecture Excalidraw diagram."""
import json
import uuid

def uid():
    return str(uuid.uuid4())

def make_rect(x, y, w, h, bg_color, stroke_color, label, font_size=20, group_id=None):
    rect_id = uid()
    text_id = uid()
    elements = []
    rect_el = {
        "id": rect_id, "type": "rectangle",
        "x": x, "y": y, "width": w, "height": h,
        "angle": 0, "strokeColor": stroke_color,
        "backgroundColor": bg_color, "fillStyle": "solid",
        "strokeWidth": 2, "roughness": 1, "opacity": 100,
        "groupIds": [group_id] if group_id else [],
        "roundness": {"type": 3}, "seed": 123456,
        "version": 1, "versionNonce": 123456,
        "isDeleted": False,
        "boundElements": [{"id": text_id, "type": "text"}],
        "updated": 1, "link": None, "locked": False
    }
    elements.append(rect_el)
    text_el = {
        "id": text_id, "type": "text",
        "x": x + 10, "y": y + h/2 - font_size/2 - 2,
        "width": w - 20, "height": font_size + 4,
        "angle": 0, "strokeColor": "#ffffff",
        "backgroundColor": "transparent", "fillStyle": "solid",
        "strokeWidth": 1, "roughness": 1, "opacity": 100,
        "groupIds": [group_id] if group_id else [],
        "roundness": None, "seed": 123456,
        "version": 1, "versionNonce": 123456,
        "isDeleted": False, "boundElements": None,
        "updated": 1, "link": None, "locked": False,
        "text": label, "fontSize": font_size, "fontFamily": 1,
        "textAlign": "center", "verticalAlign": "middle",
        "containerId": rect_id, "originalText": label,
        "autoResize": True, "lineHeight": 1.25
    }
    elements.append(text_el)
    return elements

def make_arrow(x1, y1, x2, y2, color="#1e1e1e", width=2):
    return {
        "id": uid(), "type": "arrow",
        "x": x1, "y": y1, "width": x2 - x1, "height": y2 - y1,
        "angle": 0, "strokeColor": color,
        "backgroundColor": "transparent", "fillStyle": "solid",
        "strokeWidth": width, "roughness": 1, "opacity": 100,
        "groupIds": [], "roundness": {"type": 2}, "seed": 123456,
        "version": 1, "versionNonce": 123456,
        "isDeleted": False, "boundElements": None,
        "updated": 1, "link": None, "locked": False,
        "points": [[0, 0], [x2 - x1, y2 - y1]],
        "lastCommittedPoint": None, "startBinding": None,
        "endBinding": None, "startArrowhead": None,
        "endArrowhead": "arrow"
    }

def make_text(x, y, text, font_size=20, color="#1e1e1e"):
    return {
        "id": uid(), "type": "text",
        "x": x, "y": y,
        "width": len(text) * font_size * 0.55,
        "height": font_size + 4,
        "angle": 0, "strokeColor": color,
        "backgroundColor": "transparent", "fillStyle": "solid",
        "strokeWidth": 1, "roughness": 1, "opacity": 100,
        "groupIds": [], "roundness": None, "seed": 123456,
        "version": 1, "versionNonce": 123456,
        "isDeleted": False, "boundElements": None,
        "updated": 1, "link": None, "locked": False,
        "text": text, "fontSize": font_size, "fontFamily": 1,
        "textAlign": "left", "verticalAlign": "top",
        "originalText": text, "autoResize": True, "lineHeight": 1.25
    }

# LAYOUT (20px grid aligned)
GW_X, GW_Y = 60, 80
GW_W, GW_H = 340, 60
EP_X, EP_W, EP_H = 80, 300, 50
EP_GAP = 80
W_X, W_W, W_H = 750, 300, 55
W_GAP = 85
ARROW_SX = EP_X + EP_W
ARROW_EX = W_X
EP_YS = [200, 280, 360, 440]
W_YS = [60, 145, 230, 315, 400]

elements = []

# Title
elements.append(make_text(350, 15, "ZeroClaw Dashboard Architecture", 28, "#1971c2"))
elements.append(make_text(120, 55, "BACKEND", 16, "#868e96"))

# Gateway block
elements.extend(make_rect(GW_X, GW_Y, GW_W, GW_H, "#1971c2", "#1e1e1e", "ZeroClaw Gateway :42617", 20, "gw"))

# Endpoint blocks
eps = [
    ("/v1/events/stream", "SSE - eventy JSON co 7s"),
    ("/api/chat/sessions", "GET - lista sesji chatu"),
    ("POST /v1/chat/sessions", "POST - tworzenie sesji"),
    ("/api/status", "GET - status systemu"),
]
for i, (label, desc) in enumerate(eps):
    y = EP_YS[i]
    elements.extend(make_rect(EP_X, y, EP_W, EP_H, "#228be6", "#1e1e1e", label, 16, f"ep{i}"))
    elements.append(make_text(EP_X + 10, y + EP_H + 2, desc, 12, "#868e96"))

# Widgets
elements.append(make_text(810, 35, "DASHBOARD WIDGETY", 16, "#868e96"))
wds = [("CommandCenter", "#e8590c"), ("LiveActivity", "#5c940d"),
       ("GraphicDesigner", "#862e9c"), ("CareerConcierge", "#1864ab"),
       ("SystemStatus", "#495057")]
for i, (name, color) in enumerate(wds):
    elements.extend(make_rect(W_X, W_YS[i], W_W, W_H, color, "#1e1e1e", name, 18, f"w{i}"))

# Arrows: SSE -> all widgets (blue)
sse_cy = EP_YS[0] + EP_H / 2
for i in range(5):
    elements.append(make_arrow(ARROW_SX, sse_cy, ARROW_EX, W_YS[i] + W_H / 2, "#228be6", 2))

# Arrows: API -> widgets (green)
elements.append(make_arrow(ARROW_SX, EP_YS[1] + EP_H/2, ARROW_EX, W_YS[0] + W_H/2, "#40c057", 2))
elements.append(make_arrow(ARROW_SX, EP_YS[2] + EP_H/2, ARROW_EX, W_YS[0] + W_H/2, "#40c057", 2))
elements.append(make_arrow(ARROW_SX, EP_YS[3] + EP_H/2, ARROW_EX, W_YS[4] + W_H/2, "#40c057", 2))

# Legend
LX, LY = 450, 500
elements.append(make_text(LX, LY - 30, "LEGENDA", 16, "#1e1e1e"))
elements.append(make_arrow(LX, LY + 5, LX + 60, LY + 5, "#228be6", 3))
elements.append(make_text(LX + 70, LY - 5, "SSE (Server-Sent Events)", 14, "#228be6"))
elements.append(make_arrow(LX, LY + 35, LX + 60, LY + 35, "#40c057", 3))
elements.append(make_text(LX + 70, LY + 25, "REST API Call", 14, "#40c057"))

data = {
    "type": "excalidraw", "version": 2,
    "source": "zeroclaw-dashboard-architecture",
    "elements": elements,
    "appState": {"gridSize": 20, "gridStep": 5, "gridModeEnabled": True, "viewBackgroundColor": "#ffffff"},
    "files": {}
}

path = "/home/arndtos/zeroclaw-dashboard-architecture.excalidraw"
with open(path, "w", encoding="utf-8") as f:
    json.dump(data, f, indent=2, ensure_ascii=False)
print(f"Saved: {path} ({len(elements)} elements)")
