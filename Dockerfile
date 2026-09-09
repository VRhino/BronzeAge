# Imagen de despliegue del backend, genérica (Render, Fly, Railway, Koyeb, cualquier host de contenedores).
# No hay paso de build: el servidor corre TypeScript directo con `tsx` (por eso `tsx` está en `dependencies`,
# no en `devDependencies`).
FROM node:22-slim

ENV NODE_ENV=production
WORKDIR /app

# Capa de dependencias: se reinstala solo si cambian los manifiestos.
COPY package.json package-lock.json ./
RUN npm ci --omit=dev

# El código. `.dockerignore` deja fuera tests, cliente/, lab/, Docs/, partidas/, etc.
COPY . .

# `PORT` lo inyecta la plataforma; el server ya lo lee (index.ts). Documental:
EXPOSE 3000

CMD ["npm", "run", "server"]
