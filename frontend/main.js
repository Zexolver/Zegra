// main.js
document.addEventListener("DOMContentLoaded", function () {
    var navItems = document.querySelectorAll("nav li");
    var pages = document.querySelectorAll(".page");
  
    navItems.forEach(function (item) {
      item.addEventListener("click", function () {
        navItems.forEach(function (i) {
          i.classList.remove("active");
        });
        item.classList.add("active");
  
        var target = item.getAttribute("data-page");
        pages.forEach(function (page) {
          if (page.id === target) {
            page.classList.remove("hidden");
          } else {
            page.classList.add("hidden");
          }
        });
      });
    });
  });
  