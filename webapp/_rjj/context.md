# Charms Indexer WebApp

## Project Overview
The Charms Indexer WebApp is a Next.js application that displays information about charms (NFTs, tokens, and dApps) from the Charms blockchain. It fetches data from an API and displays it in a user-friendly interface.

## Key Components
- **src/services/transformers.js**: Transforms raw charm data from the API into a format suitable for the UI
- **src/services/apiUtils.js**: Utility functions for API operations, including nested property extraction
- **src/services/apiServices.js**: Core API service functions for fetching charm data
- **src/services/apiConfig.js**: API configuration and endpoints
- **src/components/CharmCard.js**: Component for displaying charm cards in the asset grid
- **src/app/asset/[id]/page.js**: Page for displaying detailed information about a specific asset

## Database Structure
The application fetches data from an API and doesn't directly interact with a database.

## Recent Changes
- Enhanced metadata handling to support the new charms metadata standard
- Updated transformCharmData function to extract all fields from the new JSON structure
- Enhanced getNestedProperty function to handle array access in paths
- Updated asset detail page to display additional metadata fields
- Added version tag to CharmCard component
- Updated createDefaultCharm to include new fields
- Added image hash verification feature with simplified approach
  - Displays clear error message about CORS restrictions
  - Maintains verification status indicator in the UI
  - Provides informative message about why verification cannot be performed
  - Allows manual verification attempt with a "Verify Hash" button
  - Simplified implementation to avoid browser security errors

## Useful Commands
```bash
# Start the development server
npm run dev

# Build the application
npm run build

# Start the production server
npm start

# Lint the code
npm run lint
```

## Docker Commands
```bash
# Build the Docker image
docker build -t charms-explorer-webapp .

# Run the Docker container
docker run -p 3000:3000 charms-explorer-webapp
```

## Makefile
The project includes a Makefile with common commands. See the Makefile for details.

## Notes
- The application uses the new charms metadata standard which includes:
  - Input UTXO IDs
  - App data
  - Output charms data (url, name, image, ticker, image_hash, description, remaining)
  - Version information
- The metadata is extracted from the nested structure in the JSON response
- The UI displays different information based on the charm type (NFT, token, dApp)
