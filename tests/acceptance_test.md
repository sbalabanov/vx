# Acceptance Test for VX

This document describes the acceptance test for the VX version control system.

## Overview

The acceptance test (`acceptance_test.sh`) verifies that the core functionality of the VX version control system works correctly. The test executes a series of commands to simulate a typical workflow and verifies that the expected outcomes are achieved.

## Test Workflow

The test follows this workflow:

1. **Build the VX binary** - Ensures that the code builds successfully
2. **Create a new repository** - Tests the `repo new` command
3. **Add files and folders** - Creates test files in the repository
4. **Check status** - Tests the `tree status` command to view changes
5. **Create initial commit** - Tests the `commit new` command
6. **Checkout the commit** - Tests the `tree checkout` command
7. **Create a new branch** - Tests the `branch new` command
8. **Modify files** - Makes changes to existing files and adds new files
9. **Commit changes on the feature branch** - Tests committing changes
10. **Switch to initial commit** - Tests switching back to the original commit
11. **Verify initial commit content** - Ensures files match the expected state
12. **Switch to feature branch commit** - Tests switching to the feature branch
13. **Verify feature branch content** - Ensures files match the expected state

## Expected Outcomes

At each step, the test verifies that:
- Commands execute without errors
- The repository state is as expected
- Files and their contents reflect the correct state after operations

## Running the Test

To run the acceptance test:

```bash
chmod +x tests/acceptance_test.sh
./tests/acceptance_test.sh
```

The test creates a temporary directory, builds the VX binary, and runs all test steps in that directory. If any step fails, it will exit with an error code and display which step failed. On success, it will display "ALL TESTS PASSED" and clean up the temporary directory.

## Test Output

The test output will show each step as it is executed with colorized output:
- Blue: Test step descriptions
- Green: Success messages
- Red: Failure messages

## Troubleshooting

If the test fails:
1. Check the error message to identify which step failed
2. Verify that the VX binary built successfully
3. Inspect the command that failed and its parameters
4. Check that the expected output matches the actual output

## Test Maintenance

When making changes to the VX application:
1. Update the test if command formats change
2. Add new tests if new functionality is added
3. Ensure parsing of command output matches the current format 
