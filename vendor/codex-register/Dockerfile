FROM python:3.10-slim

ARG PLAYWRIGHT_BROWSER_INSTALL_ARGS="chromium"

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
    wget \
    gnupg \
    && rm -rf /var/lib/apt/lists/*

# Install real Google Chrome so register flows can switch off Playwright's
# bundled Chromium for better anti-automation fingerprint.
RUN wget -q -O - https://dl.google.com/linux/linux_signing_key.pub \
        | gpg --dearmor -o /usr/share/keyrings/google-chrome.gpg \
    && echo "deb [arch=amd64 signed-by=/usr/share/keyrings/google-chrome.gpg] http://dl.google.com/linux/chrome/deb/ stable main" \
        > /etc/apt/sources.list.d/google-chrome.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends google-chrome-stable \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy requirements first for better caching
COPY requirements.txt .

# Install Python dependencies
RUN pip install --no-cache-dir -r requirements.txt

ENV PLAYWRIGHT_BROWSERS_PATH=/ms-playwright

# Install full Chromium by default so Docker deployments can switch from
# headless automation to headed noVNC handoff without rebuilding a different
# image. Builders can still override this ARG for headless-only scenarios.
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
