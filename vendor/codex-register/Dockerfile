FROM python:3.10-slim

ARG PLAYWRIGHT_BROWSER_INSTALL_ARGS="chromium --only-shell"

# Install system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    fonts-liberation \
    libasound2 \
    libatk-bridge2.0-0 \
    libatk1.0-0 \
    libcups2 \
    libdbus-1-3 \
    libdrm2 \
    libgbm1 \
    libglib2.0-0 \
    libgtk-3-0 \
    libnspr4 \
    libnss3 \
    libx11-6 \
    libx11-xcb1 \
    libxcb1 \
    libxcomposite1 \
    libxdamage1 \
    libxext6 \
    libxfixes3 \
    libxkbcommon0 \
    libxrandr2 \
    novnc \
    websockify \
    x11vnc \
    xvfb \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy requirements first for better caching
COPY requirements.txt .

# Install Python dependencies
RUN pip install --no-cache-dir -r requirements.txt

ENV PLAYWRIGHT_BROWSERS_PATH=/ms-playwright

# Install the smallest Playwright runtime that still supports our headless
# browser flows by default. Builders can override this ARG when they need the
# full Chromium bundle for headed debugging.
RUN python -m playwright install ${PLAYWRIGHT_BROWSER_INSTALL_ARGS}

# Copy application code
COPY . .

# Create data directory
RUN mkdir -p data logs \
    && chmod +x /app/docker/start-register.sh

# Expose port
EXPOSE 9000 7900

# Environment variables
ENV PYTHONUNBUFFERED=1

# Run the application
CMD ["/app/docker/start-register.sh"]
