/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        grove: {
          bg: "#1a1b26",
          surface: "#24283b",
          border: "#3b4261",
          text: "#c0caf5",
          "text-muted": "#565f89",
          accent: "#7aa2f7",
          success: "#9ece6a",
          warning: "#e0af68",
          error: "#f7768e",
        },
      },
      fontFamily: {
        mono: [
          "JetBrains Mono",
          "Fira Code",
          "SF Mono",
          "Menlo",
          "monospace",
        ],
      },
    },
  },
  plugins: [],
};
