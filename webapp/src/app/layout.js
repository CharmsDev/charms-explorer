import '../styles/globals.css';
import Header from '../components/Header';
import Footer from '../components/Footer';
import { NetworkProvider } from '../context/NetworkContext';

export const metadata = {
    title: 'Charms Explorer',
    description: 'Explore Charms - NFTs, Tokens, and dApps',
};

export default function RootLayout({ children }) {
    return (
        <html lang="en" className="dark">
            <body className="min-h-screen bg-dark-950 text-white">
                {/* Background elements */}
                <div className="fixed inset-0 z-[-1] grid-bg opacity-10"></div>
                <div className="fixed inset-0 z-[-2] bg-gradient-to-br from-dark-950 via-dark-900 to-primary-950/30"></div>

                <NetworkProvider>
                    <Header />

                    {/* Background blur for header */}
                    <div className="h-16 fixed top-0 left-0 right-0 bg-dark-900/80 backdrop-blur-md z-40"></div>

                    {/* Actual spacer to create space in document flow */}
                    <div className="h-16"></div>

                    <main className="relative z-10">
                        {children}
                    </main>
                </NetworkProvider>

                <Footer />
            </body>
        </html>
    );
}
