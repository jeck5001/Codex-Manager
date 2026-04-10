const hotmailForm = document.getElementById('hotmail-batch-form');
const hotmailStatus = document.getElementById('hotmail-batch-status');

if (hotmailForm) {
    hotmailForm.addEventListener('submit', async (event) => {
        event.preventDefault();
        const payload = {
            count: parseInt(document.getElementById('hotmail-count').value, 10) || 1,
            concurrency: parseInt(document.getElementById('hotmail-concurrency').value, 10) || 1,
            interval_min: parseInt(document.getElementById('hotmail-interval-min').value, 10) || 0,
            interval_max: parseInt(document.getElementById('hotmail-interval-max').value, 10) || 0,
        };

        try {
            const data = await api.post('/hotmail/batches', payload);
            hotmailStatus.textContent = `批次已创建: ${data.batch_id}`;
        } catch (error) {
            hotmailStatus.textContent = `创建失败: ${error.message}`;
        }
    });
}
