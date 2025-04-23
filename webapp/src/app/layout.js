import '../styles/globals.css';
import Header from '../components/Header';

export const metadata = {
    title: 'Charms Explorer',
    description: 'Explore Bitcoin Charms - NFTs, Tokens, and dApps',
};

export default function RootLayout({ children }) {
    return (
        <html lang="en" className="dark">
            <body className="min-h-screen bg-dark-950 text-white">
                {/* Background elements */}
                <div className="fixed inset-0 z-[-1] grid-bg opacity-10"></div>
                <div className="fixed inset-0 z-[-2] bg-gradient-to-br from-dark-950 via-dark-900 to-primary-950/30"></div>

                <Header />

                {/* Background blur for header */}
                <div className="h-16 fixed top-0 left-0 right-0 bg-dark-900/80 backdrop-blur-md z-40"></div>

                {/* Actual spacer to create space in document flow */}
                <div className="h-12"></div>

                <main className="relative z-10">
                    {children}
                </main>

                <footer className="relative z-10 bg-dark-900/80 backdrop-blur-sm border-t border-dark-800 text-white py-8 mt-12">
                    <div className="container mx-auto px-4">
                        <div className="flex flex-col md:flex-row justify-between items-center">
                            <div className="mb-4 md:mb-0">
                                <div className="flex items-center">
                                    <img
                                        src="https://charms.dev/_astro/logo-charms-dark.Ceshk2t3.png"
                                        alt="Charms Logo"
                                        className="h-10 w-auto mr-2 animate-float"
                                    />
                                    <span className="text-xl font-bold gradient-text">Explorer</span>
                                </div>
                                <p className="text-dark-400 mt-2 text-sm">The premier explorer for Bitcoin Charms</p>
                            </div>
                            <div className="flex flex-col items-end">
                                <div className="flex space-x-4 mb-3">
                                    <a href="https://twitter.com/charmsbtc" target="_blank" rel="noopener noreferrer" className="text-dark-400 hover:text-primary-400 transition-colors">
                                        <svg className="h-5 w-5" fill="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                            <path d="M8.29 20.251c7.547 0 11.675-6.253 11.675-11.675 0-.178 0-.355-.012-.53A8.348 8.348 0 0022 5.92a8.19 8.19 0 01-2.357.646 4.118 4.118 0 001.804-2.27 8.224 8.224 0 01-2.605.996 4.107 4.107 0 00-6.993 3.743 11.65 11.65 0 01-8.457-4.287 4.106 4.106 0 001.27 5.477A4.072 4.072 0 012.8 9.713v.052a4.105 4.105 0 003.292 4.022 4.095 4.095 0 01-1.853.07 4.108 4.108 0 003.834 2.85A8.233 8.233 0 012 18.407a11.616 11.616 0 006.29 1.84"></path>
                                        </svg>
                                    </a>
                                    <a href="https://github.com/CharmsDev/charms" target="_blank" rel="noopener noreferrer" className="text-dark-400 hover:text-primary-400 transition-colors">
                                        <svg className="h-5 w-5" fill="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                            <path fillRule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" clipRule="evenodd"></path>
                                        </svg>
                                    </a>
                                    <span className="text-dark-400 cursor-not-allowed">
                                        <svg aria-hidden="true" className="h-5 w-5" width="16" height="16" viewBox="0 0 24 24" fill="currentColor" style={{ '--sl-icon-size': '1em' }}>
                                            <path d="M12 .3a12 12 0 0 0-3.8 23.38c.6.12.83-.26.83-.57L9 21.07c-3.34.72-4.04-1.61-4.04-1.61-.55-1.39-1.34-1.76-1.34-1.76-1.08-.74.09-.73.09-.73 1.2.09 1.83 1.24 1.83 1.24 1.08 1.83 2.81 1.3 3.5 1 .1-.78.42-1.31.76-1.61-2.67-.3-5.47-1.33-5.47-5.93 0-1.31.47-2.38 1.24-3.22-.14-.3-.54-1.52.1-3.18 0 0 1-.32 3.3 1.23a11.5 11.5 0 0 1 6 0c2.28-1.55 3.29-1.23 3.29-1.23.64 1.66.24 2.88.12 3.18a4.65 4.65 0 0 1 1.23 3.22c0 4.61-2.8 5.63-5.48 5.92.42.36.81 1.1.81 2.22l-.01 3.29c0 .31.2.69.82.57A12 12 0 0 0 12 .3Z"></path>
                                        </svg>
                                    </span>
                                </div>
                                <div className="text-sm text-dark-400">
                                    &copy; {new Date().getFullYear()} Charms Explorer. All rights reserved.
                                </div>
                            </div>
                        </div>
                    </div>
                </footer>
            </body>
        </html>
    );
}
