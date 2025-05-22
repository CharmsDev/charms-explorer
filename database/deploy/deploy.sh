#!/bin/bash
set -e

# Colores para output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}Iniciando despliegue de base de datos PostgreSQL en fly.io...${NC}"

# Verificar si fly CLI está instalado
if ! command -v fly &> /dev/null; then
    echo -e "${RED}Error: fly CLI no está instalado. Por favor instálalo primero:${NC}"
    echo "curl -L https://fly.io/install.sh | sh"
    exit 1
fi

# Variables de configuración
APP_NAME="charms-explorer-database"
REGION="sjc"
VOLUME_NAME="postgres_data"
VOLUME_SIZE=10
PG_USER="ch4rm5u53r"
PG_PASSWORD="8f7d56a1e2c9b3f4d6e8a7c5"
PG_DB="charms_indexer"

# Crear la aplicación en la organización Charms Inc
echo -e "${YELLOW}Creando aplicación en fly.io...${NC}"
fly apps create $APP_NAME --org "charms-inc" || echo -e "${YELLOW}La aplicación ya existe, continuando...${NC}"

# Crear volumen para datos de PostgreSQL
echo -e "${YELLOW}Creando volumen para datos...${NC}"
fly volumes create $VOLUME_NAME --size $VOLUME_SIZE --region $REGION -a $APP_NAME || echo -e "${YELLOW}El volumen ya existe, continuando...${NC}"

# Configurar variables de entorno como secretos en fly.io
echo -e "${YELLOW}Configurando variables de entorno...${NC}"
fly secrets set \
    POSTGRES_USER=$PG_USER \
    POSTGRES_PASSWORD=$PG_PASSWORD \
    POSTGRES_DB=$PG_DB \
    -a $APP_NAME

# Desplegar la aplicación
echo -e "${YELLOW}Desplegando aplicación...${NC}"
# Nos aseguramos de estar en el directorio database
cd "$(dirname "$0")/.."
fly deploy -a $APP_NAME

# Mostrar información de conexión
echo -e "${GREEN}Despliegue completado.${NC}"
echo -e "${YELLOW}Para conectarte a la base de datos:${NC}"
echo -e "URL de conexión externa: postgres://$PG_USER:$PG_PASSWORD@$APP_NAME.fly.dev:5432/$PG_DB"
echo -e "${YELLOW}Para verificar el estado:${NC}"
echo "fly status -a $APP_NAME"
