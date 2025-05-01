# Zegra Launcher

**Zegra** is a browser-based game launcher and store mockup designed to be lightweight, visually clean, and modular for expansion.

## 🎯 Purpose

Zegra is built as an educational + functional frontend experiment, designed to:

- Simulate a PC game launcher (like Steam or Epic Games)
- Offer a simple UI/UX for managing games and store content
- Work entirely in-browser for now (school Chromebook-friendly)
- Use **TypeScript/JavaScript, HTML, and CSS** (no external frameworks yet)

## ✅ Current Features

- Sidebar with tabs: **Library**, **Store**, and **Settings**
- Tab switching using `localStorage` for persistence
- Game card modal popup on click
- Responsive layout with clean, minimal UI
- Can be edited/previewed on any device with a browser

## 🛠 Planned Features

- Search bar functionality
- Game library population from external APIs
- Integration with APIs (Itch.io, Game Jolt, possibly Steam or Epic)
- Game install tracking (local only or with future backend)
- Offline-friendly data caching and save persistence

## 💡 Tech Stack

- **HTML/CSS/JS (compiled from TS)**
- Lightweight, framework-free
- Optional: Compilable via [TypeScript playground](https://www.typescriptlang.org/play)

## 📌 Notes

This project is being developed using a Chromebook, and is designed to function without needing node/npm or a build system. All files are static and browser-runnable.
