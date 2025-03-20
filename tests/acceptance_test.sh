#!/bin/bash
set -e

# Set colors for better readability
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print steps
print_step() {
    echo -e "${BLUE}===> $1${NC}"
}

# Function to check if command succeeded
check_success() {
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}SUCCESS: $1${NC}"
    else
        echo -e "${RED}FAILED: $1${NC}"
        exit 1
    fi
}

# Create a temp directory for the test
TEST_DIR=$(mktemp -d)
echo "Running test in: $TEST_DIR"
cd "$TEST_DIR"

# Build the vx binary
print_step "Building vx binary"
cd -
cargo build --release
check_success "Build vx binary"

# Get the path to the built binary
VX_PATH="$(pwd)/target/release/vx"
cd "$TEST_DIR"

# Step 1: Create a new repo
print_step "1. Creating a new repository"
"$VX_PATH" repo new test-repo
check_success "Create repo"

REPO_DIR="$TEST_DIR/test-repo"
cd "$REPO_DIR"

# Step 2: Add some files and folders
print_step "2. Adding files and folders"
mkdir -p dir1/subdir
mkdir -p empty_dir
mkdir -p dir2
echo "Hello, World!" > file1.txt
echo "Another file" > dir1/file2.txt
echo "Nested file" > dir1/subdir/file3.txt
echo "File in dir2" > dir2/file4.txt
echo "Another file in dir2" > dir2/file5.txt
check_success "Add files and folders"

# Step 3: Run "tree status" command
print_step "3. Running tree status command"
"$VX_PATH" tree status
check_success "Tree status"

# Step 4: Create initial commit and checkout
print_step "4. Creating initial commit"
COMMIT_OUTPUT=$("$VX_PATH" commit new "First commit")
echo "$COMMIT_OUTPUT"
# Extract the commit sequence number from output like "Created new commit: [seq] - [message]"
COMMIT_SEQ=$(echo "$COMMIT_OUTPUT" | grep -o "Created new commit: [0-9]*" | cut -d' ' -f4)
check_success "Create initial commit"

# Checkout the commit
print_step "5. Checking out initial commit"
"$VX_PATH" tree checkout "$COMMIT_SEQ"
check_success "Checkout initial commit"

# Step 5: Run "branch new" command
print_step "6. Creating a new branch"
"$VX_PATH" branch new feature-branch
check_success "Create new branch"

# Step 6: Modify some files
print_step "7. Modifying files"
echo "Modified content" >> file1.txt
echo "New file in branch" > new-branch-file.txt
check_success "Modify files"

# Step 7: Run "commit new" and "checkout" command again after modifications
print_step "8. Committing changes on feature branch"
NEW_COMMIT_OUTPUT=$("$VX_PATH" commit new "Feature branch changes")
echo "$NEW_COMMIT_OUTPUT"
# Extract the commit sequence number
NEW_COMMIT_SEQ=$(echo "$NEW_COMMIT_OUTPUT" | grep -o "Created new commit: [0-9]*" | cut -d' ' -f4)
check_success "Create feature branch commit"

print_step "9. Switching to initial commit"
"$VX_PATH" tree checkout main:"$COMMIT_SEQ"  # Back to initial commit
check_success "Switch to initial commit"

# Verify that the content is as expected after switching to initial commit
print_step "10. Verifying initial commit content"
if [ "$(cat file1.txt)" = "Hello, World!" ] && \
   [ "$(cat dir1/file2.txt)" = "Another file" ] && \
   [ "$(cat dir1/subdir/file3.txt)" = "Nested file" ] && \
   [ "$(cat dir2/file4.txt)" = "File in dir2" ] && \
   [ "$(cat dir2/file5.txt)" = "Another file in dir2" ] && \
   [ -d "empty_dir" ] && \
   [ ! -f new-branch-file.txt ]; then
    echo -e "${GREEN}SUCCESS: Content verification after switching to initial commit${NC}"
else
    echo -e "${RED}FAILED: Content verification after switching to initial commit${NC}"
    exit 1
fi

print_step "11. Switching to feature branch commit"
"$VX_PATH" tree checkout feature-branch:"$NEW_COMMIT_SEQ"  # Back to feature branch commit
check_success "Switch to feature branch commit"

# Verify that the content is as expected after switching to feature branch
print_step "12. Verifying feature branch content"
if grep -q "Modified content" file1.txt && [ -f new-branch-file.txt ]; then
    echo -e "${GREEN}SUCCESS: Content verification after switching to feature branch${NC}"
else
    echo -e "${RED}FAILED: Content verification after switching to feature branch${NC}"
    exit 1
fi

# Clean up
print_step "Cleaning up"
cd -
rm -rf "$TEST_DIR"
check_success "Cleanup"

echo -e "${GREEN}ALL TESTS PASSED!${NC}"
exit 0 