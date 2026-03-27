import * as htmlToImage from 'html-to-image';

export type ExportFormat = 'png' | 'jpeg' | 'svg';

export async function exportChart(
    chartElement: HTMLElement,
    filename: string,
    format: ExportFormat = 'png'
): Promise<void> {
    try {
        let dataUrl: string;

        // We apply some styling to maintain appearance during export
        const style = {
            backgroundColor: '#0f172a', // match dark theme typical bg
        };

        switch (format) {
            case 'jpeg':
                dataUrl = await htmlToImage.toJpeg(chartElement, { quality: 0.95, style });
                break;
            case 'svg':
                dataUrl = await htmlToImage.toSvg(chartElement, { style });
                break;
            case 'png':
            default:
                dataUrl = await htmlToImage.toPng(chartElement, { style });
                break;
        }

        // Create download link and trigger
        const link = document.createElement('a');
        link.download = `${filename}.${format}`;
        link.href = dataUrl;
        link.click();
    } catch (error) {
        console.error('Error exporting chart:', error);
        throw new Error('Failed to export chart image');
    }
}
