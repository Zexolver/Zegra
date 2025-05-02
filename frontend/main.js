// main.js

const sidebarItems = document.querySelectorAll("nav li");

// Load last active tab from localStorage
window.addEventListener("load", () => {
  const activeTab = localStorage.getItem("activeTab") || "Library";
  sidebarItems.forEach((item) => {
    if (item.textContent === activeTab) {
      item.classList.add("active");
    } else {
      item.classList.remove("active");
    }
  });
});

// Handle tab switching and save to localStorage
sidebarItems.forEach((item) => {
  item.addEventListener("click", () => {
    sidebarItems.forEach((el) => el.classList.remove("active"));
    item.classList.add("active");
    localStorage.setItem("activeTab", item.textContent);
    console.log(`Navigated to ${item.textContent}`);
  });
});
