// main.js

// Get the sidebar list items and the modal elements
const sidebarItems = document.querySelectorAll("nav li");
const modal = document.getElementById("game-modal");
const modalGameTitle = document.getElementById("modal-game-title");

// Function to switch between tabs
sidebarItems.forEach((item) => {
    item.addEventListener("click", () => {
        sidebarItems.forEach((el) => el.classList.remove("active"));
        item.classList.add("active");

        // Simulate navigation by logging the active tab (can be expanded to show content dynamically)
        console.log(`Navigating to ${item.textContent}...`);
    });
});

// Open game details modal
function openModal(gameTitle) {
    modalGameTitle.textContent = gameTitle;
    modal.style.display = "block";
}

// Close game details modal
function closeModal() {
    modal.style.display = "none";
}

// Check localStorage and load previous tab
window.addEventListener("load", () => {
    const activeTab = localStorage.getItem("activeTab") || "Library";
    document.querySelectorAll("nav li").forEach((item) => {
        if (item.textContent === activeTab) {
            item.classList.add("active");
        }
    });
});

// Save active tab to localStorage
sidebarItems.forEach((item) => {
    item.addEventListener("click", () => {
        localStorage.setItem("activeTab", item.textContent);
    });
});