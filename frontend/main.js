document.querySelectorAll("nav li").forEach((tab) => {
    tab.addEventListener("click", () => {
      // Remove 'active' from all
      document.querySelectorAll("nav li").forEach((el) => el.classList.remove("active"));
      tab.classList.add("active");
  
      // Hide all sections
      document.querySelectorAll(".page-section").forEach((section) => {
        section.classList.add("hidden");
      });
  
      // Show selected section
      const targetId = tab.getAttribute("data-page");
      const targetSection = document.getElementById(targetId);
      if (targetSection) {
        targetSection.classList.remove("hidden");
      }
    });
  });
  