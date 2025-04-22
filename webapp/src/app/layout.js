import '../styles/globals.css';
import Header from '../components/Header';

export const metadata = {
    title: 'Charms Explorer',
    description: 'Explore Bitcoin Charms - NFTs, Tokens, and dApps',
};

export default function RootLayout({ children }) {
    return (
        <html lang="en">
            <body className="min-h-screen bg-gray-50 dark:bg-gray-900 text-gray-900 dark:text-white">
                <Header />
                <main>
                    {children}
                </main>
                <footer className="bg-gray-900 text-white py-8 mt-12">
                    <div className="container mx-auto px-4">
                        <div className="flex flex-col md:flex-row justify-between items-center">
                            <div className="mb-4 md:mb-0">
                                <div className="flex items-center">
                                    <img
                                        src="https://charms.dev/_astro/logo-charms-dark.Ceshk2t3.png"
                                        alt="Charms Logo"
                                        className="h-8 w-auto mr-2"
                                    />
                                    <span className="text-lg font-bold">Explorer</span>
                                </div>
                            </div>
                            <div className="text-sm text-gray-400">
                                &copy; {new Date().getFullYear()} Charms Explorer. All rights reserved.
                            </div>
                        </div>
                    </div>
                </footer>
            </body>
        </html>
    );
}
