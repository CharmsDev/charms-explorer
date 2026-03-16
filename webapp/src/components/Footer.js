'use client';

import { useState } from 'react';
import ConfigModal from './ConfigModal';

export default function Footer() {
    const [showConfig, setShowConfig] = useState(false);

    return (
        <>
            <footer className="relative z-10 bg-dark-900/80 backdrop-blur-sm border-t border-dark-800 text-white py-4 mt-12">
                <div className="container mx-auto px-4">
                    <div className="flex flex-col md:flex-row items-center justify-between gap-4">
                        {/* Brand */}
                        <div className="flex items-center gap-2">
                            <img
                                src="/images/logo.png"
                                alt="Charms Logo"
                                className="h-7 w-auto"
                            />
                            <span className="text-sm font-bold">
                                <span className="text-white">Powered by</span>{' '}
                                <span className="gradient-text">Charms</span>
                            </span>
                        </div>

                        {/* Links */}
                        <div className="flex flex-wrap items-center justify-center gap-5">
                            <a
                                href="https://charms.dev"
                                target="_blank"
                                rel="noopener noreferrer"
                                className="text-dark-300 hover:text-primary-400 transition-colors text-sm"
                            >
                                Charms Protocol
                            </a>
                            <a
                                href="https://charms.dev/Charms-whitepaper.pdf"
                                target="_blank"
                                rel="noopener noreferrer"
                                className="text-dark-300 hover:text-primary-400 transition-colors text-sm"
                            >
                                White Paper
                            </a>
                        </div>

                        {/* Icons */}
                        <div className="flex items-center gap-4">
                            <button
                                onClick={() => setShowConfig(true)}
                                className="text-dark-400 hover:text-primary-400 transition-colors"
                                title="Configuration"
                            >
                                <svg className="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                                </svg>
                            </button>
                            <a
                                href="https://github.com/CharmsDev/charms"
                                target="_blank"
                                rel="noopener noreferrer"
                                className="text-dark-400 hover:text-primary-400 transition-colors"
                                title="GitHub"
                            >
                                <svg className="h-5 w-5" fill="currentColor" viewBox="0 0 24 24">
                                    <path fillRule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" clipRule="evenodd" />
                                </svg>
                            </a>
                            <a
                                href="https://x.com/CharmsDev"
                                target="_blank"
                                rel="noopener noreferrer"
                                className="text-dark-400 hover:text-primary-400 transition-colors"
                                title="X (Twitter)"
                            >
                                <svg className="h-5 w-5" fill="currentColor" viewBox="0 0 24 24">
                                    <path d="M8.29 20.251c7.547 0 11.675-6.253 11.675-11.675 0-.178 0-.355-.012-.53A8.348 8.348 0 0022 5.92a8.19 8.19 0 01-2.357.646 4.118 4.118 0 001.804-2.27 8.224 8.224 0 01-2.605.996 4.107 4.107 0 00-6.993 3.743 11.65 11.65 0 01-8.457-4.287 4.106 4.106 0 001.27 5.477A4.072 4.072 0 012.8 9.713v.052a4.105 4.105 0 003.292 4.022 4.095 4.095 0 01-1.853.07 4.108 4.108 0 003.834 2.85A8.233 8.233 0 012 18.407a11.616 11.616 0 006.29 1.84" />
                                </svg>
                            </a>
                        </div>
                    </div>
                </div>
            </footer>

            <ConfigModal isOpen={showConfig} onClose={() => setShowConfig(false)} />
        </>
    );
}
