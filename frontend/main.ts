// main.ts

// Example: Invoke a Rust function from the frontend
import { invoke } from "@tauri-apps/api/tauri";

// Optional: Handle tab navigation
function showPage(id: string) {
  const pages = document.querySelectorAll(".page");
  pages.forEach(page => {
    (page as HTMLElement).style.display = "none";
  });

  const active = document.getElementById(id);
  if (active) {
    (active as HTMLElement).style.display = "block";
  }
}

// Bind button clicks to tabs (for Steam, GOG, etc.)
document.addEventListener("DOMContentLoaded", () => {
  const buttons = document.querySelectorAll("[data-page]");
  buttons.forEach(button => {
    button.addEventListener("click", () => {
      const page = (button as HTMLElement).getAttribute("data-page");
      if (page) showPage(page);
    });
  });

  // Optional: Call a test Rust function
  invoke<string>("greet", { name: "Zegra user" })
    .then(response => {
      console.log("Rust says:", response);
    })
    .catch(console.error);
});
