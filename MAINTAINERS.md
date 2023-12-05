# MAINTAINER NOTES

This repo uses (conventional commit messages)[https://www.conventionalcommits.org/en/v1.0.0/] to automate the release versioning.

There is a helper workflow that will test the Pull Request title to ensure the that message follows the convention.

## Personal Access Token for semantic-release commit and tagging

The GITHUB_TOKEN does not allow triggering of workflows. The persisted credentials from the actions/checkout overrides the env tokens used by semantic release and therefore these credentials must be purged.

This allows the PAT to create the new version commit and tag to allow the docker workflow to create the docker images with the correct release tag versions.

### remove skip to allow tags workflow to trigger

When the tagged commit is added to the repo, we need the docker tags workflow to create the container images. If we set the ci to skip, then the workflow is not triggered.

- Workflows cannot be trigger by events that are created with the GITHUB_TOKEN. A Personal Access Token must be used to overcome this.
- Persisted credentials from the `actions/checkout` override the semantic-release ENV vars of GH_TOKEN or GITHUB_TOKEN, when set to a PAT, and also do not trigger workflows.
- The `[skip ci]` commit message also blocks the workflow running on tags

### creating a pat for use in this repo

The organisation has been setup for PATs and a SEM_REL_TOKEN was created in this repo as an action repository secret. The value of the token is from my personal access token;`OLX_SEM_REL_PAT`, expiring 30.09.2024, scoped to this repo only, with the following scopes for semantic-release to succeed.

| scope           | access     |
| --------------- | ---------- |
| metadata        | read       |
| commit statuses | read-write |
| contents        | read-write |
| issues          | read-write |
| pull requests   | read-write |

The content of the `SEM_REL_TOKEN` will need to be updated from time to time, as the PAT tokens now carry an expiry time and can be refreshed, without the need to change their scopes. Therefore, ensure to set a reminder of the expiry date and update the token value in this repo to ensure that the semantic-release process is not blocked.
