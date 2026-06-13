export interface CrawlConfig {
  url: string;
  maxDepth: number;
  pageLimit: number;
  downloadAssets: boolean;
  headlessStrategy: 'never' | 'auto' | 'always';
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
  message?: string;
  level?: string;
  progress?: CrawlProgress;
  page?: PageResult;
  status?: 'queued' | 'running' | 'paused' | 'completed' | 'failed';
}

export interface SearchMatch {
  url: string;
  title: string;
  preview: string;
  relevance: number;
}

export interface DashboardStats {
  pagesSaved: number;
  totalSizeBytes: number;
  crawlVelocity: number;
  failRate: number;
}

export interface RecentExport {
  path: string;
  jobId: string;
  createdAt: string;
  sizeBytes: number;
}