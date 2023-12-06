/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    './src/**/*.rs',
    "./templates/**/*.{html,html.j2}",
    "./public/**/*.{html,js,css}"
  ],
  theme: {
    extend: {},
  },
  plugins: [],
}
