import { Workbox } from 'workbox-window';

const wb = new Workbox('/service-worker.js');
wb.register();
