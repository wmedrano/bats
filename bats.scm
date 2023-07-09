(define-module (bats)
  #:use-module (srfi srfi-1)
  #:export (
            ;; Exported by extension.
            activate-logging!
            settings
            make-track!
            delete-track!
            tracks
            plugins
            make-plugin-instance!
            delete-plugin-instance!
            plugin-instance

            ;; Scheme exports.
            track
            track-id
            track-ids
            track-plugins
            delete-all-tracks!
            plugin
            plugin-instrument-p
            instrument-plugins
            ))

;; TODO: Move the below to a module.
;; TODO: Avoid hardcoding target/debug directory.
(load-extension "target/debug/libbats" "init_bats")

(define (track-id track)
  "Get the id for the given track."
  (assoc-ref track 'track-id))

(define (track id)
  "Get the track with the given id"
  (car (filter (lambda (t) (equal? (track-id t) id))
               (tracks))))

(define (track-ids)
  "Get all track ids."
  (map track-id (tracks)))

(define (track-plugins track)
  "Get the plugins that correspond to the track's plugin instances."
  (let ((plugins          (plugins))
        (plugin-instances (map plugin-instance
                               (assoc-ref track 'plugin-instance-ids))))
    (map (lambda (plugin-instance)
           (let ((plugin-id (assoc-ref plugin-instance 'plugin-id)))
             (car (filter (lambda (plugin) (equal? plugin-id (assoc-ref plugin
                                                                        'plugin-id)))
                          plugins))))
         plugin-instances)))


(define (delete-all-tracks!)
  "Delete all the tracks."
  (count identity
         (map delete-track! (track-ids))))

(define (plugin id)
  "Get the metadata for the plugin with the given id."
  (car (filter (lambda (plugin) (equal? id (assoc-ref plugin 'plugin-id)))
               (plugins))))

(define (plugin-instrument-p plugin)
  "Returns #t if plugin is an instrument or #f otherwise."
  (assoc-ref plugin 'instrument?))

(define (instrument-plugins)
  "Get all plugins that are instruments."
  (filter plugin-instrument-p (plugins)))
