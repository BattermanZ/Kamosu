/**
 * The Job envelope, which is the same for every Job there will ever be.
 *
 * Asking for slow work answers a job id at once; state, progress and result are
 * read back through the ordinary Operations `get_job` and `list_jobs`
 * (ADR 0032). That shape is the one thing a Job's own declaration does not
 * describe — its declared output is its eventual result — so it is written here
 * once rather than generated per Operation. Everything else about a Job, its
 * states included, comes from the generated declarations.
 */

/** What asking for a Job answers, immediately, whatever the Job is. */
export interface JobAsk {
	job_id: string;
}
