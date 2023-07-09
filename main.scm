(use-modules (srfi srfi-1))

;; TODO: Move the below to a module.
;; TODO: Avoid hardcoding target/debug directory.
(load-extension "target/debug/libbats" "init_bats")

(let ()
  (activate-logging!)
  (settings))

(define (delete-all-tracks!)
  (count identity
  (map delete-track! (track-ids))))

(define (instrument-plugins)
  (filter (lambda (p) (assoc-ref p 'instrument?))
          (plugins)))

(define (tracks)
  (map track (track-ids)))

(define (make-track-with-any-instrument!)
  (let ((instrument (car (instrument-plugins))))
    (make-track! #:plugins (list (assoc-ref instrument 'id)))))
