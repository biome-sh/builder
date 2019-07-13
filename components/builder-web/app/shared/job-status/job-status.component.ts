import { Component, Input } from '@angular/core';

@Component({
  selector: 'bio-job-status',
  template: `
    <bio-job-status-icon [job]="job" [animate]="true"></bio-job-status-icon>
    <bio-job-status-label [job]="job"></bio-job-status-label>
  `
})
export class JobStatusComponent {

  @Input() job: any;
}
