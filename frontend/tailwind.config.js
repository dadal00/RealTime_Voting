/** @type {import('tailwindcss').Config} */
export default {
  content: ["./src/**/*.{html,js,svelte,ts}"],

  theme: {
    extend: {}
  },

  plugins: []
};

const config = {
  content: [
    "./src/**/*.{html,js,svelte,ts}",
    "./node_modules/flowbite-svelte/**/*.{html,js,svelte,ts}",
  ],

  plugins: [
    require('flowbite/plugin')
  ],

  darkMode: 'class',
  
  theme: {
    extend: {
      colors: {
        // flowbite-svelte
        primary: { 50: '#FFF5F2', 100: '#FFF1EE', 200: '#FFE4DE', 300: '#FFD5CC', 400: '#FFBCAD', 500: '#FE795D', 600: '#EF562F', 700: '#EB4F27', 800: '#CC4522', 900: '#A5371B'},
      },
      animation: {
        meteor: "meteor 5s linear infinite",
        gradient: "gradient 5s ease-in-out infinite",
      },
      keyframes: {
        meteor: {
          "0%": { transform: "rotate(215deg) translateX(0)", opacity: 1 },
          "70%": { opacity: 1 },
          "100%": {
            transform: "rotate(215deg) translateX(-100vw)",
            opacity: 0,
          },
        },
        gradient: {
          "0%": { "background-position": "0% center" },
          "50%": { "background-position": "200% center" },
          "100%": { "background-position": "0% center" },
        },
      },
    }
  }
};

module.exports = config;
