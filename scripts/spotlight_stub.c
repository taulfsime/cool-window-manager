// spotlight_stub.c - minimal executable for cwm Spotlight shortcuts
// compiled once at build time and embedded in cwm binary
//
// this stub reads the command from a sibling file (cwm_command.txt)
// and sends it to the cwm daemon via Unix socket

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <libgen.h>
#include <mach-o/dyld.h>
#include <errno.h>

#define SOCKET_PATH_TEMPLATE "%s/.cwm/cwm.sock"
#define COMMAND_FILE "cwm_command.txt"
#define MAX_CMD_LEN 4096
#define MAX_PATH_LEN 2048
#define RECV_TIMEOUT_SEC 2

// shows an error dialog using osascript
static void show_error_dialog(const char *title, const char *message) {
    char cmd[MAX_CMD_LEN];
    snprintf(cmd, sizeof(cmd),
        "osascript -e 'display dialog \"%s\" buttons {\"OK\"} default button \"OK\" with title \"%s\"' 2>/dev/null",
        message, title);
    system(cmd);
}

// shows a notification using osascript
static void show_notification(const char *title, const char *message) {
    char cmd[MAX_CMD_LEN];
    snprintf(cmd, sizeof(cmd),
        "osascript -e 'display notification \"%s\" with title \"%s\"' 2>/dev/null",
        message, title);
    system(cmd);
}

// gets the path to the command file next to this executable
static int get_command_file_path(char *out_path, size_t out_size) {
    char exe_path[MAX_PATH_LEN];
    uint32_t size = sizeof(exe_path);
    
    if (_NSGetExecutablePath(exe_path, &size) != 0) {
        return -1;
    }
    
    // resolve symlinks
    char real_path[MAX_PATH_LEN];
    if (realpath(exe_path, real_path) == NULL) {
        strncpy(real_path, exe_path, sizeof(real_path) - 1);
        real_path[sizeof(real_path) - 1] = '\0';
    }
    
    // get directory
    char *dir = dirname(real_path);
    snprintf(out_path, out_size, "%s/%s", dir, COMMAND_FILE);
    
    return 0;
}

// reads the command from the command file
static int read_command(const char *cmd_path, char *out_cmd, size_t out_size) {
    FILE *f = fopen(cmd_path, "r");
    if (!f) {
        return -1;
    }
    
    if (!fgets(out_cmd, out_size, f)) {
        fclose(f);
        return -1;
    }
    
    fclose(f);
    
    // trim trailing newline
    size_t len = strlen(out_cmd);
    while (len > 0 && (out_cmd[len-1] == '\n' || out_cmd[len-1] == '\r')) {
        out_cmd[--len] = '\0';
    }
    
    return 0;
}

// gets the socket path (~/.cwm/cwm.sock)
static int get_socket_path(char *out_path, size_t out_size) {
    const char *home = getenv("HOME");
    if (!home) {
        return -1;
    }
    snprintf(out_path, out_size, SOCKET_PATH_TEMPLATE, home);
    return 0;
}

// sends command to daemon and receives response
static int send_to_daemon(const char *socket_path, const char *command, char *response, size_t resp_size) {
    int sock = socket(AF_UNIX, SOCK_STREAM, 0);
    if (sock < 0) {
        return -1;
    }
    
    struct sockaddr_un addr;
    memset(&addr, 0, sizeof(addr));
    addr.sun_family = AF_UNIX;
    strncpy(addr.sun_path, socket_path, sizeof(addr.sun_path) - 1);
    
    if (connect(sock, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        close(sock);
        return -1;
    }
    
    // set receive timeout
    struct timeval tv;
    tv.tv_sec = RECV_TIMEOUT_SEC;
    tv.tv_usec = 0;
    setsockopt(sock, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof(tv));
    
    // send command with newline
    char cmd_with_newline[MAX_CMD_LEN];
    snprintf(cmd_with_newline, sizeof(cmd_with_newline), "%s\n", command);
    
    if (send(sock, cmd_with_newline, strlen(cmd_with_newline), 0) < 0) {
        close(sock);
        return -1;
    }
    
    // receive response
    ssize_t n = recv(sock, response, resp_size - 1, 0);
    if (n > 0) {
        response[n] = '\0';
    } else {
        response[0] = '\0';
    }
    
    close(sock);
    return 0;
}

int main(int argc, char *argv[]) {
    char cmd_path[MAX_PATH_LEN];
    char command[MAX_CMD_LEN];
    char socket_path[MAX_PATH_LEN];
    char response[MAX_CMD_LEN];
    
    // get command file path
    if (get_command_file_path(cmd_path, sizeof(cmd_path)) != 0) {
        show_error_dialog("cwm Error", "Failed to locate command file");
        return 1;
    }
    
    // read command
    if (read_command(cmd_path, command, sizeof(command)) != 0) {
        show_error_dialog("cwm Error", "Failed to read command file");
        return 1;
    }
    
    // get socket path
    if (get_socket_path(socket_path, sizeof(socket_path)) != 0) {
        show_error_dialog("cwm Error", "Failed to determine socket path");
        return 1;
    }
    
    // check if socket exists
    if (access(socket_path, F_OK) != 0) {
        show_error_dialog("cwm Error", 
            "cwm daemon is not running.\\n\\nStart it with:\\n  cwm daemon start\\n\\nOr enable auto-start:\\n  cwm daemon install");
        return 1;
    }
    
    // send to daemon
    if (send_to_daemon(socket_path, command, response, sizeof(response)) != 0) {
        show_error_dialog("cwm Error", 
            "Failed to connect to cwm daemon.\\n\\nTry restarting it:\\n  cwm daemon stop\\n  cwm daemon start");
        return 1;
    }
    
    // check response for errors
    if (strstr(response, "\"error\"") != NULL) {
        // extract error message if possible
        char *msg_start = strstr(response, "\"message\"");
        if (msg_start) {
            msg_start = strchr(msg_start, ':');
            if (msg_start) {
                msg_start++;
                while (*msg_start == ' ' || *msg_start == '"') msg_start++;
                char *msg_end = strchr(msg_start, '"');
                if (msg_end) {
                    *msg_end = '\0';
                    show_notification("cwm Error", msg_start);
                    return 1;
                }
            }
        }
        show_notification("cwm Error", "Command failed");
        return 1;
    }
    
    return 0;
}
