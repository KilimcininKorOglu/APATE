# Apate — Branding Guide

## 1. Name & Identity

### 1.1 Project Name

- **Name**: Apate
- **Pronunciation**: ah-PAH-teh
- **Etymology**: From Ἀπάτη, Greek personification of deception; reflects stealth and misdirection against traffic analysis and active probing.
- **In code**: `apate`
- **In prose**: `Apate` (capitalized sentence case)

### 1.2 Tagline

- **Primary**: Stealth tunnel protocol for hostile networks
- **Technical**: Low-latency Rust VPN protocol with DPI evasion
- **Marketing**: Stay connected where network control fights back

### 1.3 Elevator Pitch

Apate is a high-performance stealth VPN protocol for restrictive network environments. It combines low-latency encrypted transport with traffic camouflage and active probing defense, while keeping a strict minimal-dependency engineering model. Apate targets operators and security engineers who need protocol-level control, not black-box VPN tooling.

## 2. Logo

### 2.1 Concept

Primary metaphor: a split-mask sigil inside a shield-ring.  
Mask symbolizes deception and camouflage. Shield-ring symbolizes resilient encrypted transport.

### 2.2 Specifications

- **Primary mark**: Shield outline + abstract split-mask glyph + subtle packet trail line
- **Icon mark**: Mask glyph only (centered in circular frame)
- **Wordmark**: `APATE` in uppercase geometric sans
- **Minimum size**: 20px icon, 96px full lockup
- **Clear space**: At least `0.5x` icon height on each side

### 2.3 AI Generation Prompt

“Design minimal cybersecurity protocol logo for ‘Apate’. Flat vector style, no gradients, no text. Central abstract split-mask symbol inside shield outline, subtle network packet trail motif. Palette limited to deep navy, cobalt blue, and cyan accent from provided brand colors. High contrast on dark and light backgrounds. Clean geometric lines, modern, technical, precise. Produce square icon composition (1:1) and wide banner composition (16:9). Avoid mascots, 3D effects, glossy rendering, and ornamental details.”

## 3. Color Palette

### 3.1 Brand Colors

| Role      | Name        | Hex     | RGB              | Usage |
|-----------|-------------|---------|------------------|-------|
| Primary   | Abyss Navy  | #0B1220 | rgb(11, 18, 32)  | Main surfaces, hero backgrounds |
| Secondary | Cobalt Flux | #1D4ED8 | rgb(29, 78, 216) | Links, primary actions |
| Accent    | Signal Cyan | #22D3EE | rgb(34, 211, 238)| Highlights, active indicators |

### 3.2 Neutrals

| Role            | Hex     | Usage |
|-----------------|---------|-------|
| Text Primary    | #E6EDF7 | Main body text on dark surfaces |
| Text Secondary  | #9FB0C8 | Supporting labels and meta text |
| Background      | #070B14 | App/page background |
| Surface         | #111827 | Cards, panes, CLI panels |
| Border          | #223045 | Dividers and control borders |

### 3.3 Semantic Colors

| Role    | Hex     | Usage |
|---------|---------|-------|
| Success | #22C55E | Healthy tunnel/session states |
| Error   | #EF4444 | Failures, auth rejection, hard errors |
| Warning | #F59E0B | Degraded network states |
| Info    | #38BDF8 | Informational telemetry |

### 3.4 Dark Mode

Brand defaults are dark-first.  
For light mode: swap background/surface to `#F8FAFC` / `#FFFFFF`, switch text primary to `#0F172A`, keep primary and accent unchanged.

### 3.5 CSS Variables

```css
:root {
  --color-primary: #0B1220;
  --color-secondary: #1D4ED8;
  --color-accent: #22D3EE;
  --color-bg: #070B14;
  --color-surface: #111827;
  --color-text: #E6EDF7;
  --color-text-secondary: #9FB0C8;
  --color-border: #223045;
  --color-success: #22C55E;
  --color-error: #EF4444;
  --color-warning: #F59E0B;
  --color-info: #38BDF8;
}
```

## 4. Typography

### 4.1 Font Stack

| Role     | Font                      | Weights      | Fallback |
|----------|---------------------------|--------------|----------|
| Headings | Space Grotesk             | 600, 700     | Inter, system-ui, sans-serif |
| Body     | Inter                     | 400, 500     | system-ui, -apple-system, sans-serif |
| Code     | JetBrains Mono            | 400, 500     | ui-monospace, SFMono-Regular, monospace |

### 4.2 Type Scale

| Element | Size   | Weight | Line Height |
|---------|--------|--------|-------------|
| H1      | 2.25rem| 700    | 1.2 |
| H2      | 1.75rem| 600    | 1.25 |
| H3      | 1.375rem| 600   | 1.35 |
| Body    | 1rem   | 400    | 1.6 |
| Small   | 0.875rem| 400   | 1.5 |
| Code    | 0.9rem | 500    | 1.5 |

## 5. Voice & Tone

### 5.1 Personality

- **Precise**: Use exact technical claims, no vague promises.
- **Unflinching**: State threat assumptions and trade-offs directly.
- **Calm**: Communicate security posture without hype language.
- **Operator-first**: Favor actionable guidance over marketing flourish.

### 5.2 Writing Rules

- Headlines: short, direct, technical signal first.
- Documentation: advanced-engineering audience baseline; explain rationale and constraints.
- Error messages: include cause + action path.
- Marketing copy: outcomes first, no fear-based exaggeration.

### 5.3 Vocabulary

| Prefer                 | Avoid                    |
|------------------------|--------------------------|
| stealth profile        | magic mode               |
| authenticated session  | secure-ish connection    |
| deterministic fallback | auto magic               |
| threat model           | scary internet           |
| transport overhead     | blazing fast (unproven)  |

## 6. Visual Language

### 6.1 Border Radius

Use `8px` default, `12px` for cards, `999px` only for pills/badges.

### 6.2 Shadows

Minimal shadow system:
- `elevation-1`: `0 1px 2px rgba(0,0,0,0.35)`
- `elevation-2`: `0 6px 18px rgba(0,0,0,0.35)`

### 6.3 Spacing

Base unit `4px`, scale: `4, 8, 12, 16, 20, 24, 32, 40, 48, 64`.

### 6.4 Icons

- Library: Lucide
- Style: outline
- Stroke width: `1.75`
- Default sizes: `16`, `20`, `24`

## 7. Assets Checklist

| Asset          | Format      | Size                | Status |
|----------------|-------------|---------------------|--------|
| Logo (full)    | SVG + PNG   | vector / 1024px     | Required for v1 |
| Icon           | SVG + PNG   | 512px, 192px, 64px  | Required for v1 |
| Favicon        | ICO + PNG   | 32px, 16px          | Required for v1 |
| OG Image       | PNG         | 1200×630            | Required for v1 |
| Social Banner  | PNG         | 1500×500            | Required for v1 |
| Website Hero   | PNG/SVG     | 1600×900            | Required for v1 |
