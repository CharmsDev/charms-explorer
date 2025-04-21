/** @type {import('tailwindcss').Config} */
module.exports = {
    content: [
        './src/pages/**/*.{js,ts,jsx,tsx,mdx}',
        './src/components/**/*.{js,ts,jsx,tsx,mdx}',
        './src/app/**/*.{js,ts,jsx,tsx,mdx}',
    ],
    theme: {
        extend: {
            colors: {
                primary: '#009DFE', // celestial-blue
                blue: {
                    400: '#00ACFA', // picton-blue
                    500: '#009DFE', // celestial-blue
                    600: '#009FFC', // celestial-blue-2
                    700: '#009EFE', // celestial-blue-3
                },
                indigo: {
                    300: '#33BBFB', // lighter picton-blue
                    400: '#00ACFA', // picton-blue
                    500: '#009DFE', // celestial-blue
                    600: '#009FFC', // celestial-blue-2
                    700: '#009EFE', // celestial-blue-3
                    800: '#0080D1', // darker celestial-blue
                },
            },
        },
    },
    plugins: [],
}
