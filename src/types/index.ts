export interface CrawlConfig {
  url: string;
  maxDepth: number;
  pageLimit: number;
  downloadAssets: boolean;
  headlessStrategy: 'disabled' | 'js-only' | 'all';
  contentSelectors: string[];
  excludePatterns: string[];
  respectRobotsTxt: boolean;
  outputDir: string;
}

export interface PageResult {
  url: string;
  title: string;
  content: string;
  links: string[];
  assets: string[];
  status: number;
}

export interface CrawlProgress {
  pagesCrawled: number;
  pageLimit: number;
  currentUrl: string;
  depth: number;
  maxDepth: number;
  startTime: string;
}

export interface CrawlJob {
  id: string;
  url: string;
  status: 'queued' | 'running' | 'paused' | 'completed' | 'failed';
  config: CrawlConfig;
  progress: CrawlProgress;
  results: PageResult[];
  startTime?: string;
  endTime?: string;
  error?: string;
}

export interface AppSettings {
  outputDir: string;
  concurrency: number;
  requestDelay: number;
  timeout: number;
  userAgent: string;
  defaultMaxDepth: number;
  defaultPageLimit: number;
}

export interface CrawlEvent {
  type: 'progress' | 'log' | 'pageComplete' | 'jobStatusChanged' | 'error';
  jobId: string;
  data: any;
  timestamp: string;
}