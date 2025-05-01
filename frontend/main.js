document.addEventListener("DOMContentLoaded", () => {
    const navItems = document.querySelectorAll("nav li");
    const sections = document.querySelectorAll(".main section");
    const gameCards = document.querySelectorAll(".game-card");
  
    const modal = document.createElement("div");
    modal.className = "modal hidden";
    modal.innerHTML = `
      <div class="modal-content">
        <span class="close-button">&times;</span>
        <h2 class="modal-title">Game Title</h2>
        <img class="modal-image" src="" alt="Game" />
        <p class="modal-description">More information about this game.</p>
      </div>
    `;
    document.body.appendChild(modal);
  
    const closeModal = () => {
      modal.classList.add("hidden");
    };
  
    modal.querySelector(".close-button").addEventListener("click", closeModal);
    modal.addEventListener("click", (e) => {
      if (e.target === modal) closeModal();
    });
  
    // Handle game card click
    gameCards.forEach((card) => {
      card.addEventListener("click", () => {
        const title = card.querySelector("h3")?.innerText || "Game Title";
        const imageSrc = card.querySelector("img")?.src || "placeholder.jpg";
  
        modal.querySelector(".modal-title").innerText = title;
        modal.querySelector(".modal-image").src = imageSrc;
        modal.querySelector(".modal-description").innerText =
          "This is a placeholder description for " + title + ".";
  
        modal.classList.remove("hidden");
      });
    });
  
    // Restore last active tab from localStorage
    const savedTab = localStorage.getItem("activeTab");
    if (savedTab) {
      navItems.forEach((item) => {
        if (item.textContent === savedTab) {
          item.classList.add("active");
        } else {
          item.classList.remove("active");
        }
      });
    }
  
    navItems.forEach((item) => {
      item.addEventListener("click", () => {
        navItems.forEach((i) => i.classList.remove("active"));
        item.classList.add("active");
        localStorage.setItem("activeTab", item.textContent);
        // Optional: update view logic per tab here in the future
      });
    });
  });
  